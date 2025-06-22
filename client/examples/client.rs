use client::{Connection, Flags};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let stream = TcpStream::connect("localhost:4567").await.unwrap();
    stream.set_nodelay(true).unwrap();

    let password = "";
    let path = ":memory:";
    let flags = Flags::default();

    let mut client = Connection::connect(stream, password, path, flags)
        .await
        .unwrap();

    client.ping().await.unwrap();

    client
        .execute("create table test (id integer primary key, value text)")
        .await
        .unwrap();

    client
        .execute(
            r#"
            insert into test (value) values ('hello Dog');
            insert into test (value) values ('hello Cat');
            insert into test (value) values ('hello Monkey');
          "#,
        )
        .await
        .unwrap();

    let query = client.query("select * from test").await.unwrap();
    dbg!(&query);

    client.execute("delete from test").await.unwrap();

    client.disconnect().await.unwrap();
}
