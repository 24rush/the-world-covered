use ground_covered::App;

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_BACKTRACE", "0");

    let app = App::with_athlete(4399230).await.unwrap();
    app.start_data_pipeline().await;
}
