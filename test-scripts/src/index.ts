import * as setupContracts from './scripts/setupContracts';
import * as testContracts from './scripts/testContracts';

(async () => {
    const mode = process.env.npm_config_mode || "";
    switch (mode) {
        case "testnet_setup_mixer":
            await setupContracts.setupMixer();
            break;
        case "testnet_test_mixer":
            await testContracts.testMixer();
            break;
        case "testnet_setup_anchor":
            await setupContracts.setupAnchor();
            break;
        case "testnet_test_anchor":
            await testContracts.testAnchor();
            break;
        default:
            console.log("Invalid command");
            break;
    }
})();