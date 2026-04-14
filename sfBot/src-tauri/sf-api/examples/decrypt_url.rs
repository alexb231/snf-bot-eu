#[tokio::main]
pub async fn main() {
    
    let login_resp = Some("serverversion:2004&...");
    
    let url = "https://f8.sfgame.net/req.php?req=0...";
    let decrypted = sf_api::session::decrypt_url(url, login_resp).unwrap();
    println!("{decrypted:#?}");
}
