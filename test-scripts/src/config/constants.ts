// ---------------------------------------------------------------------------------------------------
// TestNet information
// ---------------------------------------------------------------------------------------------------
export const testnet = {
    // TestNet juno-1
    networkInfo: {
      url: "", // Add Juno testnet url here
      chainId: "juno-1",
    },

    mnemonicKeys: {
      wallet1:
        "",
      wallet2:
        "",
      wallet3: 
        "",
      wallet4:
        "",
    },
    // Should be updated contract addresses after deploying wasms in the testnet
    contracts: {
      cw20: "juno...", // Ordinary CW20 token address
      signatureBridge: "juno...",
      tokenWrapper: "juno...",
      tokenWrapperHandler: "juno...",
      anchorHandler: "juno...",
      anchor: "juno...",
      vanchor: "juno...",
      mixer: "juno...",
      treasury: "juno...",
      treasuryHandler: "juno...",
    },

  } as const;
