Certainly! Here's a refined and clearer version of your README:

# Gable Current Recovery Plan Simulation

## Overview
This guide provides instructions on how to simulate the Gable current recovery plan. This process requires the latest snapshot of the radix database. Follow the steps below to correctly set up and run the simulation.

## Getting Started

### Prerequisites
- Ensure you have `rust` and `cargo` installed on your system to run the simulation.

### Downloading the Database Snapshot
1. Visit [Radix Snapshots](https://snapshots.radix.live/) to download the latest snapshot of the radix database.
2. Unpack the downloaded snapshot to your desired location on your system.

### Setting Up the Environment Variable
1. Locate the `state_manager` directory within the unpacked snapshot.
2. Set the `STATE_MANAGER_DATABASE_PATH` environment variable to the path of the `state_manager` directory. This enables the simulation to correctly access the database. Use the following command in your terminal, replacing the path with your specific directory path:

```bash
export STATE_MANAGER_DATABASE_PATH="/path/to/your/state_manager"
```

For example:

```bash
export STATE_MANAGER_DATABASE_PATH="/home/bbarw/babylon-ledger/state_manager/"
```

### Running the Simulation
With the environment variable set, you can now run the simulation. Execute the following command in your terminal:

```bash
cargo run --release
```
