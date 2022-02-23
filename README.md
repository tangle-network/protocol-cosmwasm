<h1 align="center">Webb Protocol CosmWasm</h1>

<p align="center">
    <strong>🕸️  Webb Protocol CosmWasm!  ⧫</strong>
    <br />
    <sub> ⚠️ Beta Software ⚠️ </sub>
</p>

<br />

## Dependencies
Add the compilation target.
```
rustup target add wasm32-unknown-unknown
```

## Building
To build the contract, run the following command.
```
cargo wasm
```
You can see the output wasm file in the "target/wasm32-unknown-unknown/release" dir.

For the optimization, run the following command.
```
cargo run-script optimize
```
Then, you can see the output wasm file in the "artifacts" directory.

## Testing 
For running the unit tests(written on "src/contract.rs"), run the following command.
```
cargo test
```

## License

<sup>
Licensed under <a href="LICENSE">Apache License 2.0</a>.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the GPLV3 license, shall
be licensed as above, without any additional terms or conditions.
</sub>