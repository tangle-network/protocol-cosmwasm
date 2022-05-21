import * as LocalNet from "./environments/localjuno";
import * as TestNet from "./environments/testnet";

//----------------------------------------------------------------------------------------
// Test-suite for LocalJuno, TestNet, and MainNet
//----------------------------------------------------------------------------------------
(async () => {
	const mode = process.env.npm_config_mode || "";
	switch (mode) {
		case "localjuno_tests":
			await LocalNet.startTests();
			break;
		case "localjuno_setup_contracts":
			await LocalNet.startSetupContracts();
			break;
		case "localjuno_migrate_contracts":
			// await LocalNet.startMigrateContracts();
			break;


		case "testnet_tests":
			await TestNet.startTests();
			break;
		case "testnet_setup_contracts":
			await TestNet.startSetupContracts();
			break;
		case "testnet_migrate_contracts":
			// await TestNet.startMigrateContracts();
			break;
		default:
			console.log("Invalid command");
			break;
	}
})();
