# EchoLite

EchoLite is a simple and lightweight SQLite proxy that allows you to quickly access SQLite databases on remote servers over the network.

## Why EchoLite?

SQLite is an excellent embedded database, widely appreciated for its lightweight, high-performance, and zero-configuration nature. However, SQLite doesn't natively support network access.

EchoLite bridges this gap by providing a secure proxy layer that enables network access to SQLite databases, allowing you to work with SQLite remotely just like you would with MySQL or PostgreSQL.

## Installation

### Pre-compiled Binaries

Download pre-compiled binaries for your operating system from the [Releases](https://github.com/DataflareApp/echolite/releases) page.

> [!NOTE]
> Names containing `dynamic` = dynamically linked SQLite
> 
> Names containing `static` = statically linked SQLite

### Compile from Source

Make sure you have the Rust toolchain installed ([rustup](https://rustup.rs/)):

```bash
# Clone the repository
git clone https://github.com/DataflareApp/echolite.git
cd echolite

# Build release version
cargo build --release

# If you need to dynamically link SQLite
cargo build --release --no-default-features
```

After compilation, the binary file is located at `target/release/echolite`.

## Usage

### Basic Usage

Start the EchoLite server:

```bash
./echolite -p 'your-password'
```

By default, EchoLite binds to `127.0.0.1:4567`. You can change the bind address using the `-b` parameter:

```bash
# Change port only
./echolite -p 'your-password' -b 1234

# Change IP and port
./echolite -p 'your-password' -b 192.168.0.8:7788
```

### Log Configuration

Use the `-l` parameter to adjust the log level for more detailed output:

```bash
./echolite -p 'your-password' -l trace
```

Supported log levels: `error`, `warn`, `info`, `debug`, `trace`, `off`

### Docker Deployment

```bash
# Pull the image
docker pull dataflare/echolite

# Run the container
docker run -d \
    --name echolite \
    -p 127.0.0.1:4567:4567 \
    -e ECHOLITE_BIND='0.0.0.0' \
    -e ECHOLITE_PASSWORD='YOUR_PASSWORD' \
    -e ECHOLITE_LOG='info' \
    -v /your/database/path:/echolite/ \
    dataflare/echolite
```

### Environment Variable Configuration

EchoLite supports configuration through environment variables:

-   `ECHOLITE_BIND`: Bind address (default: `127.0.0.1:4567`)
-   `ECHOLITE_PASSWORD`: Authentication password
-   `ECHOLITE_LOG`: Log level (default: `info`)

### Security Considerations

> [!WARNING]
>
> -   **Security audit**: EchoLite has not undergone professional security audits.
> -   **Always use strong passwords**: EchoLite uses Argon2id for password hashing, but weak passwords still pose risks
> -   **Network security**: EchoLite currently doesn't support TLS, it's recommended to:
>     -   Only bind to local addresses (`127.0.0.1`)
>     -   Access remote servers through SSH tunnels or VPN
>     -   Use firewalls to restrict access in production environments
> -   **Database backups**: Regularly backup your SQLite database files

## Connecting to SQLite Database

### Using Dataflare

Dataflare has out-of-the-box support for EchoLite. You can directly create a new EchoLite connection in [Dataflare](https://dataflare.app) and easily access your SQLite database.

![Dataflare](https://github.com/user-attachments/assets/ee56dd92-b80c-4c7a-96bf-e17756f207bf)

### Programmatic Access

If you want to access programmatically, please refer to the example in `client/examples/client.rs` in the code repository.

## TODO

-   [ ] TLS
-   [ ] Better log output
