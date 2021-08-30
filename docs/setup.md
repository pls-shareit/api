# Setup

These are instructions for setting up the database, configuring the service,
downloading or building the binary, and running it.

## 1. Set up the database

Install PostgreSQL if you haven't already, and create a database and user for
the service.

 1. [Install PostgreSQL.](https://www.postgresql.org/download/)
 2. Connect to the server as the `postgres` user (eg. `psql -U postgres`).
 3. Create a new user:

    `CREATE USER shareit WITH ENCRYPTED PASSWORD 'database-password-goes-here';`.
 4. Create a database for the user:

    `CREATE DATABASE shareit WITH OWNER shareit;`

## 2. Create the config file

The config file uses the TOML format and contains information for connecting to
the database, the external URL that points to the service, and several other
options.

 1. Open the file. For example, you could put it in `/etc/shareit/shareit.toml`.
 2. Add the following to the file (replace the Discord token and Postgres
    password as appropriate):

    ```toml
    upload_dir = "/srv/shareit/shares"

    [network]
    address = "0.0.0.0"
    host = "https://shareit.example.com"

    [database]
    pass = "database-password-goes-here"
    ```

You can see the full list of options [here](./configuration.md) and an example
config file showcasing many of the options [here](./config.example.toml).

## 3a. Download an executable

You can either download an executable (this step) or compile your own (the
next step). You may want to compile your own for security reasons, or if there
is not a pre-compiled executable for your operating system.

### Linux

 1. Download the executable
    (or [click here](https://github.com/pls-shareit/api/releases/latest/download/shareitd-linux)):

    `wget https://github.com/pls-shareit/api/releases/latest/download/shareitd-linux -qO shareitd`
 2. Make the file executable:

    `chmod +x shareitd`

### MacOS

 1. Download the executable
    (or [click here](https://github.com/pls-shareit/api/releases/latest/download/shareitd-macos)):

    `wget https://github.com/pls-shareit/api/releases/latest/download/shareitd-macos -qO shareitd`
 2. Make the file executable:

    `chmod +x shareitd`

### Windows

 1. Download the executable with Power shell
    (or [click here](https://github.com/pls-shareit/api/releases/latest/download/shareitd.exe)):

    `iwr -outf clique-collector.exe https://github.com/pls-shareit/api/releases/latest/download/shareitd.exe`

## 3b. Compile an executable

If you don't want to use a pre-compiled executable, you can compile one using
Rust:

 1. [Install Rust](https://www.rust-lang.org/tools/install).
 2. Run `cargo build --release` in the same directory as this README.
 3. Your binary will be at `target/release/shareitd`.

## 4. Run the service

This is as simple as executing the binary with the path to your configuration
file as its only argument. For example, `./shareitd /etc/shareit/shareit.toml`.
You may also wish to set it up with `systemd` or another process management
system.
