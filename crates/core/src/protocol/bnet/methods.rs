use prost::Name;

use super::generated::bgs::protocol::game_utilities::v1;
use super::generated::bgs::protocol::game_utilities::v2::client as v2;

#[derive(Debug, Clone, Copy)]
pub(super) enum Method {
    GameUtilitiesV1,
    GameUtilitiesV2,
}

struct Route {
    service_hash: u32,
    method_id: u32,
    request: &'static str,
    response: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/bnet_routes.rs"));

pub(super) fn lookup(service_hash: u32, method_id: u32) -> Option<Method> {
    let key = (service_hash, method_id & 0x3fff_ffff);

    let index = ROUTES
        .binary_search_by_key(&key, |route| (route.service_hash, route.method_id))
        .ok()?;

    let route = &ROUTES[index];

    if matches::<v1::ClientRequest, v1::ClientResponse>(route) {
        Some(Method::GameUtilitiesV1)
    } else if matches::<v2::ProcessTaskRequest, v2::ProcessTaskResponse>(route) {
        Some(Method::GameUtilitiesV2)
    } else {
        None
    }
}

fn matches<Request: Name, Response: Name>(route: &Route) -> bool {
    route.request == Request::full_name() && route.response == Response::full_name()
}
