#![warn(clippy::unwrap_used)]
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use actix::{Actor, Addr, Context, Handler};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, web};
use actix_ws::{Message, Session};
use futures_util::StreamExt;
use rustrict::CensorStr;
use serde_json::{Value, json};

struct Server {
    sites: HashMap<String, Site>,
    last_comment: HashMap<String, Instant>,
}

#[derive(Default)]
struct Site {
    comments: Vec<(String, String)>,
    sessions: HashMap<String, Session>,
}

#[derive(actix::Message)]
#[rtype(result = "()")]
enum ServerMessage {
    Comment {
        username: String,
        webpage: String,
        comment: String,
    },
    SendComments(String, Session, String),
}

impl actix::Actor for Server {
    type Context = Context<Self>;
}

impl Handler<ServerMessage> for Server {
    type Result = ();
    fn handle(&mut self, msg: ServerMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ServerMessage::Comment {
                username,
                webpage,
                comment,
            } => {
                let now = Instant::now();
                let last_time = self.last_comment.get(&username);
                if let Some(&prev) = last_time {
                    let dur = now.duration_since(prev);
                    if dur < Duration::from_secs(20) {
                        if let Some(sit) = self.sites.get_mut(&webpage) {
                            if let Some(usa) = sit.sessions.get_mut(&username) {
                                let mut usa_clone = usa.clone();
                                actix::spawn(async move {
                                    let _ = usa_clone.text(json!({
                                        "event": "rate_limit",
                                        "message" : format!("Wait {} seconds to send another message", 20 - dur.as_secs())
                                    }).to_string()).await;
                                });
                            }
                        }
                    }
                }

                self.last_comment.insert(username.clone(), Instant::now());
                let sites_entry = self.sites.entry(webpage).or_default();
                sites_entry
                    .comments
                    .push((username.clone(), comment.censor().clone()));
                while sites_entry.comments.len() > 100 {
                    sites_entry.comments.remove(0);
                }

                let sessions = sites_entry.sessions.clone();
                let c = comment.censor().clone();
                for (_username, mut session) in sessions {
                    let c_clone = c.clone();
                    let username_clone = username.clone();
                    actix::spawn(async move {
                        let _ = session
                            .text(
                                json!([{"text": c_clone, "username": username_clone}]).to_string(),
                            )
                            .await;
                    });
                }
            }
            ServerMessage::SendComments(url, socket, username) => {
                let vac = self.sites.entry(url).or_default();
                vac.sessions.insert(username, socket.clone());
                let comments = vac
                    .comments
                    .iter()
                    .map(move |c| json!({"text": c.1, "username": c.0}))
                    .collect::<Vec<Value>>();
                actix::spawn(async move {
                    let _ = socket.clone().text(json!(comments).to_string()).await;
                });
            }
        }
    }
}

fn ugg(error: &str) -> HttpResponse {
    HttpResponse::Ok().json(json!({"error": error}))
}

#[get("/test")]
async fn test() -> impl Responder {
    "Hello".to_string()
}

#[get("/api/{app_key}/comments/{url}")]
async fn get_comments(
    path: web::Path<(String, String)>,
    state: web::Data<Addr<Server>>,
    req: HttpRequest,
    body: web::Payload,
) -> actix_web::Result<impl Responder> {
    let (app_key, url) = path.into_inner();
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    let vortice_response = if let Ok(r) = reqwest::Client::new()
        .get(format!(
            "https://www.vortice.app/api/apps/{}/userdata",
            app_key
        ))
        .send()
        .await
    {
        if let Ok(text) = r.text().await {
            match serde_json::from_str::<Value>(&text) {
                Ok(r) => r,
                Err(_) => return Ok(ugg("Error with Vortice Services, try again later.")),
            }
        } else {
            return Ok(ugg(
                "Error with Vortice Services Response. Please try again later.",
            ));
        }
    } else {
        return Ok(ugg("Error with Vortice ID, try again later."));
    }
    .clone();
    if vortice_response["error"].as_str().is_some() {
        return Err(actix_web::error::ErrorForbidden("Vortice Error"));
    }
    let username = {
        let username_value = vortice_response["username"].as_str();
        match username_value {
            Some(r) => r.to_string(),
            None => return Ok(ugg("Vortice Services error")),
        }
    };
    state.do_send(ServerMessage::SendComments(
        url.clone(),
        session.clone(),
        username.clone(),
    ));
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        return;
                    }
                }
                Message::Text(msg) => {
                    if let Ok(val) = serde_json::from_str::<Value>(&msg) {
                        if let Some(text) = val["text"].as_str() {
                            state.do_send(ServerMessage::Comment {
                                username: username.to_string(),
                                webpage: url.clone(),
                                comment: text.to_string(),
                            });
                        }
                    }
                }
                _ => break,
            }
        }
        let _ = session.close(None).await;
    });

    Ok(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server on http://localhost:8087");
    let server = Server {
        sites: HashMap::new(),
        last_comment: HashMap::new(),
    }
    .start();
    HttpServer::new(move || {
        App::new()
            .wrap(
                actix_cors::Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .app_data(web::Data::new(server.clone()))
            .service(test)
            .service(get_comments)
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("0.0.0.0", 8087))?
    .run()
    .await
}
