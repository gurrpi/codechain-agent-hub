use std::cell::RefCell;

use chrono;
use rand;
use rand::Rng;

use super::super::agent::SendAgentRPC;
use super::super::common_rpc_types::{CommitHash, NodeName, ShellStartCodeChainRequest, ShellUpdateCodeChainRequest};
use super::super::router::Router;
use super::super::rpc::{response, RPCError, RPCResponse};
use super::types::{
    Context, DashboardGetNetworkResponse, DashboardNode, Log, LogGetRequest, LogGetResponse, LogGetTypesResponse,
    NodeConnection, NodeGetInfoResponse,
};

pub fn add_routing(router: &mut Router<Context>) {
    router.add_route("ping", Box::new(ping as fn(Context) -> RPCResponse<String>));
    router.add_route(
        "node_getInfo",
        Box::new(node_get_info as fn(Context, (String,)) -> RPCResponse<NodeGetInfoResponse>),
    );
    router.add_route(
        "dashboard_getNetwork",
        Box::new(dashboard_get_network as fn(Context) -> RPCResponse<DashboardGetNetworkResponse>),
    );
    router.add_route(
        "node_start",
        Box::new(node_start as fn(Context, (String, ShellStartCodeChainRequest)) -> RPCResponse<()>),
    );
    router.add_route("node_stop", Box::new(node_stop as fn(Context, (String,)) -> RPCResponse<()>));
    router.add_route("node_update", Box::new(node_update as fn(Context, (NodeName, CommitHash)) -> RPCResponse<()>));
    router.add_route(
        "shell_getCodeChainLog",
        Box::new(shell_get_codechain_log as fn(Context, (String,)) -> RPCResponse<String>),
    );
    router.add_route("log_getTypes", Box::new(log_get_types as fn(Context) -> RPCResponse<LogGetTypesResponse>));
    router.add_route("log_get", Box::new(log_get as fn(Context, (LogGetRequest,)) -> RPCResponse<LogGetResponse>));
}

fn ping(_: Context) -> RPCResponse<String> {
    response("pong".to_string())
}

fn dashboard_get_network(context: Context) -> RPCResponse<DashboardGetNetworkResponse> {
    let agents_state = context.db_service.get_agents_state();
    let connections = context.db_service.get_connections();
    let dashboard_nodes = agents_state.iter().map(|agent| DashboardNode::from_db_state(agent)).collect();
    response(DashboardGetNetworkResponse {
        nodes: dashboard_nodes,
        connections: connections.iter().map(|connection| NodeConnection::from_connection(connection)).collect(),
    })
}

fn node_get_info(context: Context, args: (String,)) -> RPCResponse<NodeGetInfoResponse> {
    let (name,) = args;
    let agent_query_result = context.db_service.get_agent_query_result(&name).ok_or(RPCError::AgentNotFound)?;
    let extra = context.db_service.get_agent_extra(&name);
    response(NodeGetInfoResponse::from_db_state(&agent_query_result, &extra))
}

fn node_start(context: Context, args: (NodeName, ShellStartCodeChainRequest)) -> RPCResponse<()> {
    let (name, req) = args;

    let agent = context.agent_service.get_agent(name.clone());
    if agent.is_none() {
        return Err(RPCError::AgentNotFound)
    }
    let agent = agent.expect("Already checked");
    agent.shell_start_codechain(req.clone())?;

    context.db_service.save_start_option(&name, &req.env, &req.args);

    response(())
}

fn node_stop(context: Context, args: (String,)) -> RPCResponse<()> {
    let (name,) = args;

    let agent = context.agent_service.get_agent(name);
    if agent.is_none() {
        return Err(RPCError::AgentNotFound)
    }
    let agent = agent.expect("Already checked");
    agent.shell_stop_codechain()?;

    response(())
}

fn node_update(context: Context, args: (NodeName, CommitHash)) -> RPCResponse<()> {
    let (name, commit_hash) = args;

    let agent = context.agent_service.get_agent(name.clone());
    if agent.is_none() {
        return Err(RPCError::AgentNotFound)
    }
    let agent = agent.expect("Already checked");

    let extra = context.db_service.get_agent_extra(&name);
    agent.shell_update_codechain(ShellUpdateCodeChainRequest {
        env: extra.as_ref().map(|extra| extra.prev_env.clone()).unwrap_or("".to_string()),
        args: extra.as_ref().map(|extra| extra.prev_args.clone()).unwrap_or("".to_string()),
        commit_hash,
    })?;

    response(())
}

fn shell_get_codechain_log(context: Context, args: (String,)) -> RPCResponse<String> {
    let (name,) = args;

    let agent = context.agent_service.get_agent(name);
    if agent.is_none() {
        return Err(RPCError::AgentNotFound)
    }
    let agent = agent.expect("Already checked");
    let result = agent.shell_get_codechain_log()?;

    response(result)
}

fn log_get_types(_context: Context) -> RPCResponse<LogGetTypesResponse> {
    response(LogGetTypesResponse {
        types: vec!["miner".to_string(), "tendermint".to_string(), "engine".to_string()],
    })
}

fn log_get(_context: Context, args: (LogGetRequest,)) -> RPCResponse<LogGetResponse> {
    let (req,) = args;
    let item_per_page = req.item_per_page.unwrap_or(100);
    let logs = (1..item_per_page).map(|_| create_dummy_log()).collect();
    response(LogGetResponse {
        logs,
    })
}

thread_local!(static dummy_id: RefCell<i32> = RefCell::new(0));

fn create_dummy_log() -> Log {
    dummy_id.with(|id_cell| {
        *id_cell.borrow_mut() += 1;
        let mut rng = rand::thread_rng();
        Log {
            id: format!("{}", *id_cell.borrow()),
            node_name: rng.choose(&vec!["node1".to_string(), "node2".to_string()]).unwrap().clone(),
            level: rng.choose(&vec!["error".to_string(), "warn".to_string()]).unwrap().clone(),
            target: rng.choose(&vec!["miner".to_string(), "tendermint".to_string()]).unwrap().clone(),
            timestamp: chrono::Local::now(),
            message: rng.choose(&vec!["Log example".to_string(), "Log another example".to_string()]).unwrap().clone(),
        }
    })
}
