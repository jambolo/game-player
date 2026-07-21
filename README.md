# game-player

This crate provides the base components for implementing a player in a two-person game.

This is a **WORK IN PROGRESS**

`develop` branch: ![Rust](https://github.com/jambolo/game-player/actions/workflows/rust.yml/badge.svg?branch=develop) [![codecov](https://codecov.io/gh/jambolo/game-player/branch/develop/graph/badge.svg)](https://codecov.io/gh/jambolo/game-player)

## Overview

The game-player crate provides the core traits and search components needed to build a player for a two-person game.

It provides:

1. A min-max game tree search algorithm using alpha-beta pruning and transposition tables for optimal performance.

## Documentation

- **[User's Guide](docs/users-guide.md)** — overview of the library, criteria for choosing a search algorithm, an
  explanation of the minimax search, and a walkthrough of a complete tic-tac-toe player.
- **API reference** — `cargo doc --open`.
- **Runnable example** — [examples/tic_tac_toe.rs](examples/tic_tac_toe.rs), via `cargo run --example tic_tac_toe`.

## Future Development

- Monte Carlo Tree Search (MCTS) with UCT-based node selection
