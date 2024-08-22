use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub type DbClient = Client<Compat<TcpStream>>;

/// Builds a connection for a database
pub async fn connect_to_sn() -> tiberius::Result<DbClient> {
    let config = {
        let mut config = Config::new();
        config.host(std::env::var("SndbServer").unwrap());
        config.database(std::env::var("SndbDatabase").unwrap());

        // use windows authentication
        config.authentication(AuthMethod::Integrated);
        config.trust_cert();

        config
    };

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    // To be able to use Tokio's tcp, we're using the `compat_write` from
    // the `TokioAsyncWriteCompatExt` to get a stream compatible with the
    // traits from the `futures` crate.
    let client = Client::connect(config, tcp.compat_write()).await?;

    Ok(client)
}
