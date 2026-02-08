use utoipa::OpenApi;

mod api {
    pub mod openapi {
        include!("../../api/openapi.rs");
    }
    pub mod handlers {
        include!("../../api/handlers.rs");
    }
}
mod cache {
    pub mod manager {
        include!("../../cache/manager.rs");
    }
}
mod models {
    pub mod card {
        include!("../../models/card.rs");
    }
}

use api::openapi::ApiDoc;

fn main() {
    let openapi = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&openapi).expect("Failed to serialize OpenAPI");
    println!("{}", json);
}
