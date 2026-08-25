# Jigsaw

Jigsaw is a simple work in progress custom Torrent client.

The main focus of this project is to write a custom Torrent client from ground up without using any pre-existing Torrent implementations.

## Project structure

The project is organized in a Cargo workspace, having multiple crates:

- `bencode` - Bencode parsing, serialization and deserialization library
- `core` - Core processes and drivers, torrent engine, tracker client, peer orchestration - everything that runs
- `cli` - The CLI frontend binary

Project will scale more horizontally in the future with more features being added
