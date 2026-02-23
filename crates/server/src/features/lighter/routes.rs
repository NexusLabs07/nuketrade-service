// use axum::routing::{get, post};

// use crate::{
//     AppState,
//     features::lighter::controller::{bridge_to_pacifica, get_user_open_positions},
// };

// pub fn routes() -> axum::Router<AppState> {
//     axum::Router::new()
//         .route(
//             "/open-positions/{user_solana_address}",
//             get(get_user_open_positions),
//         )
//         .route("/deposit", post(bridge_to_pacifica))
// }
