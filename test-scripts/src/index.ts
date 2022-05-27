import * as LocalNet from "./environments/localjuno";

//----------------------------------------------------------------------------------------
// Test-suite for LocalJuno
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
		default:
			console.log("Invalid command");
			break;
	}
})();
