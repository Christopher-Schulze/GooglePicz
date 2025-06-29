use httptest::{matchers::*, responders::*, Expectation, Server};
use serde_json::json;

/// Create a mock server for the OAuth token endpoint.
/// The server will respond to POST `/token` with a fixed access token.
pub fn token_server(access_token: &str) -> Server {
    let server = Server::run();
    server.expect(
        Expectation::matching(all_of![
            request::method_path("POST", "/token"),
        ])
        .respond_with(json_encoded(json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": 3600
        }))),
    );
    server
}

/// Create an empty mock server for Google Photos API endpoints.
pub fn photos_server() -> Server {
    Server::run()
}

fn media_items_body() -> serde_json::Value {
    let item = json!({
        "id": "1",
        "description": "desc",
        "productUrl": "https://example.com/photo",
        "baseUrl": "https://example.com/base",
        "mimeType": "image/jpeg",
        "mediaMetadata": {
            "creationTime": "2023-01-01T00:00:00Z",
            "width": "100",
            "height": "200"
        },
        "filename": "file.jpg"
    });

    json!({
        "mediaItems": [item],
        "nextPageToken": null
    })
}

/// Expect a GET request to `/v1/mediaItems` on the provided server.
pub fn expect_list(server: &Server) {
    server.expect(
        Expectation::matching(all_of![
            request::method("GET"),
            request::path("/v1/mediaItems"),
        ])
        .respond_with(json_encoded(media_items_body())),
    );
}

/// Expect a POST request to `/v1/mediaItems:search` on the provided server.
pub fn expect_search(server: &Server) {
    server.expect(
        Expectation::matching(all_of![
            request::method("POST"),
            request::path("/v1/mediaItems:search"),
        ])
        .respond_with(json_encoded(media_items_body())),
    );
}

