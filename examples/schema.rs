use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cosmwasm_mixer::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg, DepositMsg, WithdrawMsg};
use cosmwasm_mixer::state::{Mixer, MerkleTree};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(DepositMsg), &out_dir);
    export_schema(&schema_for!(WithdrawMsg), &out_dir);
    export_schema(&schema_for!(Mixer), &out_dir);
    export_schema(&schema_for!(MerkleTree), &out_dir);
}
