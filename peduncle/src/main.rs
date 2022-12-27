mod config {
    use serde::Deserialize;

    #[derive(Debug, Default, Deserialize)]
    pub struct ExampleConfig {
        pub server_addr: String,
        pub pg: deadpool_postgres::Config,
    }
}

mod models {
    use serde::{Deserialize, Serialize};
    use tokio_pg_mapper_derive::PostgresMapper;

    #[derive(Deserialize, PostgresMapper, Serialize)]
    #[pg_mapper(table = "users")]
    pub struct User {
        pub username: String,
        pub first_name: String,
        pub last_name: String,
        pub pwd: String,
    }
}

mod errors {
    use actix_web::{HttpResponse, ResponseError};
    use deadpool_postgres::PoolError;
    use derive_more::{Display, From};
    use tokio_pg_mapper::Error as PGMError;
    use tokio_postgres::error::Error as PGError;

    #[derive(Display, From, Debug)]
    pub enum Error {
        NotFound,
        PGError(PGError),
        PGMError(PGMError),
        PoolError(PoolError),
    }

    impl std::error::Error for Error {}

    impl ResponseError for Error {
        fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
            match *self {
                Error::NotFound => HttpResponse::NotFound().finish(),
                Error::PoolError(ref err) => {
                    HttpResponse::InternalServerError().body(err.to_string())
                }
                Error::PGError(ref err) => match err.code().unwrap().code() {
                    "23505" => HttpResponse::Conflict().finish(),
                    _ => HttpResponse::InternalServerError().finish(),
                },
                _ => HttpResponse::InternalServerError().finish(),
            }
        }
    }
}

mod db {
    use actix::fut::future::Map;
    use deadpool_postgres::Client;
    use tokio_pg_mapper::FromTokioPostgresRow;

    use crate::{errors::Error, models::User};

    pub async fn add_user(client: &Client, user_info: User) -> Result<User, Error> {
        let sql = include_str!("./sql/add_user.sql");
        let stmt = client
            .prepare(&sql.replace("$table_fields", &User::sql_table_fields()))
            .await
            .unwrap();

        client
            .query(
                &stmt,
                &[
                    &user_info.username,
                    &user_info.first_name,
                    &user_info.last_name,
                    &user_info.pwd,
                ],
            )
            .await?
            .iter()
            .map(|row| User::from_row_ref(row).unwrap())
            .collect::<Vec<User>>()
            .pop()
            .ok_or(Error::NotFound)
    }

    pub async fn del_user(client: &Client, username: &str) -> Result<(), Error> {
        let sql = include_str!("./sql/del_user.sql");
        let stmt = client
            .prepare(&sql.replace("$table_fields", &User::sql_table_fields()))
            .await
            .unwrap();

        client.query(&stmt, &[&username]).await?;
        Ok(())
    }
}

mod handlers {
    use actix_web::{web, Error as ActixWebError, HttpResponse};
    use deadpool_postgres::{Client, Pool};
    use serde::Deserialize;

    use crate::{db, errors::Error, models::User};

    #[derive(Deserialize)]
    pub struct Username {
        username: String,
    }

    pub async fn add_user(
        user: web::Json<User>,
        db_pool: web::Data<Pool>,
    ) -> Result<HttpResponse, ActixWebError> {
        let user_info: User = user.into_inner();
        let client: Client = db_pool.get().await.map_err(Error::PoolError)?;

        let new_user = db::add_user(&client, user_info).await?;
        Ok(HttpResponse::Ok().json(new_user))
    }

    pub async fn del_user(
        req: web::Query<Username>,
        db_pool: web::Data<Pool>,
    ) -> Result<HttpResponse, ActixWebError> {
        let client: Client = db_pool.get().await.map_err(Error::PoolError)?;
        db::del_user(&client, &req.username).await?;

        Ok(HttpResponse::Ok().finish())
    }
}

use ::config::Config;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use handlers::{add_user, del_user};
use tokio_postgres::NoTls;

use crate::config::ExampleConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let conf: ExampleConfig = Config::builder()
        .add_source(::config::Environment::default())
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();

    let pool = conf.pg.create_pool(None, NoTls).unwrap();

    let server = HttpServer::new(move || {
        App::new().app_data(web::Data::new(pool.clone())).service(
            web::resource("/users")
                .route(web::post().to(add_user))
                .route(web::delete().to(del_user)),
        )
    })
    .bind(conf.server_addr.clone())?
    .run();

    println!("server running at https://{}/", conf.server_addr);

    server.await
}
