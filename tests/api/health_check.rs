use reqwest::StatusCode;

#[tokio::test]
async fn returns_200_ok() {
    let test_address = crate::helpers::spawn_app().await;
    let response = reqwest::Client::new()
        .get(format!("{}/health-check", test_address))
        .send()
        .await
        .expect("failed request");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(Some(0), response.content_length());
}