extern crate actix;
extern crate actix_web;
extern crate bytes;
extern crate futures;
extern crate h2;
extern crate http;
extern crate tokio_timer;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::io;
use std::time::{Duration, Instant};

use actix_web::*;
use bytes::Bytes;
use futures::Future;
use http::StatusCode;
use serde_json::Value;
use tokio_timer::Delay;

#[derive(Deserialize)]
struct PParam {
    username: String,
}

#[test]
fn test_path_extractor() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.with(|p: Path<PParam>| format!("Welcome {}!", p.username))
        });
    });

    // client request
    let request = srv.get().uri(srv.url("/test/index.html")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test!"));
}

#[test]
fn test_async_handler() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with(|p: Path<PParam>| {
                Delay::new(Instant::now() + Duration::from_millis(10))
                    .and_then(move |_| Ok(format!("Welcome {}!", p.username)))
                    .responder()
            })
        });
    });

    // client request
    let request = srv.get().uri(srv.url("/test/index.html")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test!"));
}

#[test]
fn test_query_extractor() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/index.html", |r| {
            r.with(|p: Query<PParam>| format!("Welcome {}!", p.username))
        });
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/index.html?username=test"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test!"));

    // client request
    let request = srv.get().uri(srv.url("/index.html")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[derive(Deserialize, Debug)]
pub enum ResponseType {
    Token,
    Code,
}

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    id: u64,
    response_type: ResponseType,
}

#[test]
fn test_query_enum_extractor() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/index.html", |r| {
            r.with(|p: Query<AuthRequest>| format!("{:?}", p.into_inner()))
        });
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/index.html?id=64&response_type=Code"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(
        bytes,
        Bytes::from_static(b"AuthRequest { id: 64, response_type: Code }")
    );

    let request = srv
        .get()
        .uri(srv.url("/index.html?id=64&response_type=Co"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let request = srv
        .get()
        .uri(srv.url("/index.html?response_type=Code"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_async_extractor_async() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with(|data: Json<Value>| {
                Delay::new(Instant::now() + Duration::from_millis(10))
                    .and_then(move |_| Ok(format!("{}", data.0)))
                    .responder()
            })
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"{\"test\":1}"));
}

#[derive(Deserialize, Serialize)]
struct FormData {
    username: String,
}

#[test]
fn test_form_extractor() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route()
                .with(|form: Form<FormData>| format!("{}", form.username))
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html"))
        .form(FormData {
            username: "test".to_string(),
        }).unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"test"));
}

#[test]
fn test_form_extractor2() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with_config(
                |form: Form<FormData>| format!("{}", form.username),
                |cfg| {
                    cfg.0.error_handler(|err, _| {
                        error::InternalError::from_response(
                            err,
                            HttpResponse::Conflict().finish(),
                        ).into()
                    });
                },
            );
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body("918237129hdk:D:D:D:D:D:DjASHDKJhaswkjeq")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_client_error());
}

#[test]
fn test_path_and_query_extractor() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with(|(p, q): (Path<PParam>, Query<PParam>)| {
                format!("Welcome {} - {}!", p.username, q.username)
            })
        });
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html?username=test2"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test1 - test2!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_path_and_query_extractor2() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route()
                .with(|(_r, p, q): (HttpRequest, Path<PParam>, Query<PParam>)| {
                    format!("Welcome {} - {}!", p.username, q.username)
                })
        });
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html?username=test2"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test1 - test2!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_path_and_query_extractor2_async() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with(
                |(p, _q, data): (Path<PParam>, Query<PParam>, Json<Value>)| {
                    Delay::new(Instant::now() + Duration::from_millis(10))
                        .and_then(move |_| {
                            Ok(format!("Welcome {} - {}!", p.username, data.0))
                        }).responder()
                },
            )
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html?username=test2"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test1 - {\"test\":1}!"));
}

#[test]
fn test_path_and_query_extractor3_async() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with(|(p, data): (Path<PParam>, Json<Value>)| {
                Delay::new(Instant::now() + Duration::from_millis(10))
                    .and_then(move |_| {
                        Ok(format!("Welcome {} - {}!", p.username, data.0))
                    }).responder()
            })
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());
}

#[test]
fn test_path_and_query_extractor4_async() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with(|(data, p): (Json<Value>, Path<PParam>)| {
                Delay::new(Instant::now() + Duration::from_millis(10))
                    .and_then(move |_| {
                        Ok(format!("Welcome {} - {}!", p.username, data.0))
                    }).responder()
            })
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());
}

#[test]
fn test_path_and_query_extractor2_async2() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with(
                |(p, data, _q): (Path<PParam>, Json<Value>, Query<PParam>)| {
                    Delay::new(Instant::now() + Duration::from_millis(10))
                        .and_then(move |_| {
                            Ok(format!("Welcome {} - {}!", p.username, data.0))
                        }).responder()
                },
            )
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html?username=test2"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test1 - {\"test\":1}!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_path_and_query_extractor2_async3() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route()
                .with(|data: Json<Value>, p: Path<PParam>, _: Query<PParam>| {
                    Delay::new(Instant::now() + Duration::from_millis(10))
                        .and_then(move |_| {
                            Ok(format!("Welcome {} - {}!", p.username, data.0))
                        }).responder()
                })
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html?username=test2"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test1 - {\"test\":1}!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_path_and_query_extractor2_async4() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route()
                .with(|data: (Json<Value>, Path<PParam>, Query<PParam>)| {
                    Delay::new(Instant::now() + Duration::from_millis(10))
                        .and_then(move |_| {
                            Ok(format!("Welcome {} - {}!", data.1.username, (data.0).0))
                        }).responder()
                })
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html?username=test2"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test1 - {\"test\":1}!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_scope_and_path_extractor() {
    let mut srv = test::TestServer::with_factory(move || {
        App::new().scope("/sc", |scope| {
            scope.resource("/{num}/index.html", |r| {
                r.route()
                    .with(|p: Path<(usize,)>| format!("Welcome {}!", p.0))
            })
        })
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/sc/10/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome 10!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/sc/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_nested_scope_and_path_extractor() {
    let mut srv = test::TestServer::with_factory(move || {
        App::new().scope("/sc", |scope| {
            scope.nested("/{num}", |scope| {
                scope.resource("/{num}/index.html", |r| {
                    r.route().with(|p: Path<(usize, usize)>| {
                        format!("Welcome {} {}!", p.0, p.1)
                    })
                })
            })
        })
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/sc/10/12/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome 10 12!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/sc/10/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[cfg(actix_impl_trait)]
fn test_impl_trait(
    data: (Json<Value>, Path<PParam>, Query<PParam>),
) -> impl Future<Item = String, Error = io::Error> {
    Delay::new(Instant::now() + Duration::from_millis(10))
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "timeout"))
        .and_then(move |_| Ok(format!("Welcome {} - {}!", data.1.username, (data.0).0)))
}

#[cfg(actix_impl_trait)]
fn test_impl_trait_err(
    _data: (Json<Value>, Path<PParam>, Query<PParam>),
) -> impl Future<Item = String, Error = io::Error> {
    Delay::new(Instant::now() + Duration::from_millis(10))
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "timeout"))
        .and_then(move |_| Err(io::Error::new(io::ErrorKind::Other, "other")))
}

#[cfg(actix_impl_trait)]
#[test]
fn test_path_and_query_extractor2_async4_impl_trait() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with_async(test_impl_trait)
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html?username=test2"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"Welcome test1 - {\"test\":1}!"));

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test1/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[cfg(actix_impl_trait)]
#[test]
fn test_path_and_query_extractor2_async4_impl_trait_err() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/{username}/index.html", |r| {
            r.route().with_async(test_impl_trait_err)
        });
    });

    // client request
    let request = srv
        .post()
        .uri(srv.url("/test1/index.html?username=test2"))
        .header("content-type", "application/json")
        .body("{\"test\": 1}")
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_non_ascii_route() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/中文/index.html", |r| r.f(|_| "success"));
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/中文/index.html"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(bytes, Bytes::from_static(b"success"));
}

#[test]
fn test_unsafe_path_route() {
    let mut srv = test::TestServer::new(|app| {
        app.resource("/test/{url}", |r| {
            r.f(|r| format!("success: {}", &r.match_info()["url"]))
        });
    });

    // client request
    let request = srv
        .get()
        .uri(srv.url("/test/http%3A%2F%2Fexample.com"))
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    // read response
    let bytes = srv.execute(response.body()).unwrap();
    assert_eq!(
        bytes,
        Bytes::from_static(b"success: http:%2F%2Fexample.com")
    );
}
