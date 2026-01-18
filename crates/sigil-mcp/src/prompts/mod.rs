//! Sigil MCP Prompt definitions
//!
//! Prompts provide guided workflows for common signing operations.

use crate::protocol::{
    Prompt, PromptArgument, PromptContent, PromptMessage, PromptRole, PromptsGetResult,
};
use std::collections::HashMap;

/// Get all prompt definitions
pub fn get_all_prompts() -> Vec<Prompt> {
    vec![
        Prompt {
            name: "sign_evm_transfer".to_string(),
            title: Some("Sign EVM Transfer".to_string()),
            description: Some(
                "Guided workflow for signing an EVM token transfer transaction".to_string(),
            ),
            arguments: Some(vec![
                PromptArgument {
                    name: "to_address".to_string(),
                    description: Some("Recipient address (0x...)".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "amount".to_string(),
                    description: Some(
                        "Amount to transfer (in native units, e.g., '0.1')".to_string(),
                    ),
                    required: Some(true),
                },
                PromptArgument {
                    name: "chain_id".to_string(),
                    description: Some("Chain ID (default: 1 for Ethereum)".to_string()),
                    required: Some(false),
                },
            ]),
        },
        Prompt {
            name: "sign_bitcoin_taproot".to_string(),
            title: Some("Sign Bitcoin Taproot Transaction".to_string()),
            description: Some(
                "Guided workflow for signing a Bitcoin Taproot transaction using FROST".to_string(),
            ),
            arguments: Some(vec![
                PromptArgument {
                    name: "to_address".to_string(),
                    description: Some("Recipient Bitcoin address (bc1p...)".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "amount_sats".to_string(),
                    description: Some("Amount in satoshis".to_string()),
                    required: Some(true),
                },
            ]),
        },
        Prompt {
            name: "sign_solana_transfer".to_string(),
            title: Some("Sign Solana Transfer".to_string()),
            description: Some(
                "Guided workflow for signing a Solana SOL transfer using Ed25519".to_string(),
            ),
            arguments: Some(vec![
                PromptArgument {
                    name: "to_address".to_string(),
                    description: Some("Recipient Solana address".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "amount_sol".to_string(),
                    description: Some("Amount in SOL".to_string()),
                    required: Some(true),
                },
            ]),
        },
        Prompt {
            name: "troubleshoot_disk".to_string(),
            title: Some("Troubleshoot Disk Issues".to_string()),
            description: Some("Diagnose and resolve common Sigil disk problems".to_string()),
            arguments: None,
        },
        Prompt {
            name: "check_signing_readiness".to_string(),
            title: Some("Check Signing Readiness".to_string()),
            description: Some("Verify that the system is ready for signing operations".to_string()),
            arguments: None,
        },
    ]
}

/// Get a specific prompt with arguments
pub fn get_prompt(
    name: &str,
    arguments: Option<&HashMap<String, serde_json::Value>>,
) -> Result<PromptsGetResult, String> {
    match name {
        "sign_evm_transfer" => get_sign_evm_transfer_prompt(arguments),
        "sign_bitcoin_taproot" => get_sign_bitcoin_taproot_prompt(arguments),
        "sign_solana_transfer" => get_sign_solana_transfer_prompt(arguments),
        "troubleshoot_disk" => get_troubleshoot_disk_prompt(),
        "check_signing_readiness" => get_check_signing_readiness_prompt(),
        _ => Err(format!("Unknown prompt: {}", name)),
    }
}

fn get_sign_evm_transfer_prompt(
    arguments: Option<&HashMap<String, serde_json::Value>>,
) -> Result<PromptsGetResult, String> {
    let args = arguments.ok_or("Missing arguments for sign_evm_transfer")?;

    let to_address = args
        .get("to_address")
        .and_then(|v| v.as_str())
        .ok_or("Missing required argument: to_address")?;

    let amount = args
        .get("amount")
        .and_then(|v| v.as_str())
        .ok_or("Missing required argument: amount")?;

    let chain_id = args.get("chain_id").and_then(|v| v.as_u64()).unwrap_or(1);

    let chain_name = match chain_id {
        1 => "Ethereum Mainnet",
        137 => "Polygon",
        42161 => "Arbitrum One",
        10 => "Optimism",
        8453 => "Base",
        _ => "Unknown Chain",
    };

    let prompt_text = format!(
        r#"Sign an EVM transfer of {amount} to {to_address} on {chain_name} (chain ID: {chain_id}).

## Pre-flight Checks

1. **Check disk status** using `sigil_check_disk`
   - Verify a disk is inserted
   - Confirm presignatures are available
   - Check disk is not expired

2. **Get sender address** using `sigil_get_address` with format "evm"
   - Note the address for balance verification

## Transaction Details

- **To**: {to_address}
- **Amount**: {amount}
- **Chain**: {chain_name} (ID: {chain_id})

## Signing Steps

1. **Build the transaction** with:
   - Correct nonce from the network
   - Appropriate gas limit and price
   - The recipient and amount

2. **Compute the transaction hash** (keccak256 of RLP-encoded transaction)

3. **Sign** using `sigil_sign_evm`:
   ```json
   {{
     "message_hash": "0x<computed_hash>",
     "chain_id": {chain_id},
     "description": "Transfer {amount} to {to_address}"
   }}
   ```

4. **Combine signature** with the unsigned transaction

5. **Broadcast** to the network

6. **Record the transaction** using `sigil_update_tx_hash`:
   ```json
   {{
     "presig_index": <from_sign_result>,
     "tx_hash": "0x<broadcast_result>"
   }}
   ```

## Important Notes

- Each signature consumes one presignature from the disk
- Always verify transaction details before signing
- The disk must remain inserted during signing"#,
        amount = amount,
        to_address = to_address,
        chain_name = chain_name,
        chain_id = chain_id
    );

    Ok(PromptsGetResult {
        description: Some(format!(
            "Sign EVM transfer of {} to {} on {}",
            amount, to_address, chain_name
        )),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: PromptContent::Text { text: prompt_text },
        }],
    })
}

fn get_sign_bitcoin_taproot_prompt(
    arguments: Option<&HashMap<String, serde_json::Value>>,
) -> Result<PromptsGetResult, String> {
    let args = arguments.ok_or("Missing arguments for sign_bitcoin_taproot")?;

    let to_address = args
        .get("to_address")
        .and_then(|v| v.as_str())
        .ok_or("Missing required argument: to_address")?;

    let amount_sats = args
        .get("amount_sats")
        .and_then(|v| v.as_u64())
        .ok_or("Missing required argument: amount_sats")?;

    let amount_btc = amount_sats as f64 / 100_000_000.0;

    let prompt_text = format!(
        r#"Sign a Bitcoin Taproot transaction sending {amount_sats} sats ({amount_btc:.8} BTC) to {to_address}.

## Pre-flight Checks

1. **Check disk status** using `sigil_check_disk`
   - Verify disk scheme is "taproot"
   - Confirm presignatures are available

2. **Get sender address** using `sigil_get_address`:
   ```json
   {{
     "scheme": "taproot",
     "format": "bitcoin"
   }}
   ```

## Transaction Details

- **To**: {to_address}
- **Amount**: {amount_sats} satoshis ({amount_btc:.8} BTC)
- **Network**: Bitcoin Mainnet (Taproot)

## Signing Steps

1. **Build the unsigned PSBT** with:
   - UTXO inputs from the sender address
   - Output to recipient
   - Change output if needed
   - Appropriate fee rate

2. **Compute the sighash** for each input (BIP-341 Taproot sighash)

3. **Sign** using `sigil_sign_frost`:
   ```json
   {{
     "scheme": "taproot",
     "message_hash": "0x<sighash>",
     "description": "Send {amount_sats} sats to {to_address}"
   }}
   ```

4. **Finalize the PSBT** with the signature

5. **Broadcast** to the Bitcoin network

## Important Notes

- Taproot signatures are 64 bytes (BIP-340 Schnorr)
- The disk must have scheme "taproot" to sign
- FROST provides threshold security (2-of-2)"#,
        amount_sats = amount_sats,
        amount_btc = amount_btc,
        to_address = to_address
    );

    Ok(PromptsGetResult {
        description: Some(format!(
            "Sign Bitcoin Taproot transfer of {} sats to {}",
            amount_sats, to_address
        )),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: PromptContent::Text { text: prompt_text },
        }],
    })
}

fn get_sign_solana_transfer_prompt(
    arguments: Option<&HashMap<String, serde_json::Value>>,
) -> Result<PromptsGetResult, String> {
    let args = arguments.ok_or("Missing arguments for sign_solana_transfer")?;

    let to_address = args
        .get("to_address")
        .and_then(|v| v.as_str())
        .ok_or("Missing required argument: to_address")?;

    let amount_sol = args
        .get("amount_sol")
        .and_then(|v| v.as_f64())
        .ok_or("Missing required argument: amount_sol")?;

    let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;

    let prompt_text = format!(
        r#"Sign a Solana transfer of {amount_sol} SOL ({amount_lamports} lamports) to {to_address}.

## Pre-flight Checks

1. **Check disk status** using `sigil_check_disk`
   - Verify disk scheme is "ed25519"
   - Confirm presignatures are available

2. **Get sender address** using `sigil_get_address`:
   ```json
   {{
     "scheme": "ed25519",
     "format": "solana"
   }}
   ```

## Transaction Details

- **To**: {to_address}
- **Amount**: {amount_sol} SOL ({amount_lamports} lamports)
- **Network**: Solana Mainnet

## Signing Steps

1. **Build the transaction** with:
   - System program transfer instruction
   - Recent blockhash
   - Fee payer (sender)

2. **Serialize the transaction message**

3. **Sign** using `sigil_sign_frost`:
   ```json
   {{
     "scheme": "ed25519",
     "message_hash": "0x<serialized_message_hash>",
     "description": "Transfer {amount_sol} SOL to {to_address}"
   }}
   ```

4. **Attach signature** to the transaction

5. **Submit** to Solana RPC

## Important Notes

- Ed25519 signatures are 64 bytes
- Solana addresses are base58 encoded
- The disk must have scheme "ed25519" to sign"#,
        amount_sol = amount_sol,
        amount_lamports = amount_lamports,
        to_address = to_address
    );

    Ok(PromptsGetResult {
        description: Some(format!(
            "Sign Solana transfer of {} SOL to {}",
            amount_sol, to_address
        )),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: PromptContent::Text { text: prompt_text },
        }],
    })
}

fn get_troubleshoot_disk_prompt() -> Result<PromptsGetResult, String> {
    let prompt_text = r#"Diagnose and resolve common Sigil disk problems.

## Diagnostic Steps

1. **Check if daemon is running**:
   - The sigil-daemon must be running to detect disks

2. **Check disk status** using `sigil_check_disk`
   - If `detected: false` → Disk not inserted or not recognized
   - If `is_valid: false` → Disk may be corrupted or expired

3. **Common Issues and Solutions**:

### Disk Not Detected
- Ensure the floppy disk is properly inserted
- Check if the disk is mounted (look for /media/*/SIGIL*)
- Verify udev rules are configured for disk detection
- Try removing and reinserting the disk

### Disk Invalid
- Check `days_until_expiry` - disk may be expired
- Verify the disk wasn't tampered with
- The mother signature may be invalid
- Generate a new disk from the mother device

### No Presignatures Remaining
- The disk has been fully consumed
- Generate a new disk with fresh presignatures
- Consider creating disks with more presignatures (e.g., 2000 instead of 1000)

### Scheme Mismatch
- Different signature schemes are not compatible
- Create a disk with the appropriate scheme:
  - `ecdsa` for Ethereum/EVM
  - `taproot` for Bitcoin Taproot
  - `ed25519` for Solana/Cosmos
  - `ristretto255` for Zcash shielded

### Reconciliation Required
- Too many signatures since last reconciliation
- Connect the disk to the mother device to reconcile

## Advanced Diagnostics

- Check daemon logs: `journalctl -u sigil-daemon`
- Verify disk format: The disk should have a `sigil.disk` file
- Check permissions: Ensure the daemon has read/write access to the mount point"#;

    Ok(PromptsGetResult {
        description: Some("Troubleshoot common Sigil disk issues".to_string()),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: PromptContent::Text {
                text: prompt_text.to_string(),
            },
        }],
    })
}

fn get_check_signing_readiness_prompt() -> Result<PromptsGetResult, String> {
    let prompt_text = r#"Verify the system is ready for signing operations.

## Readiness Checklist

1. **Check disk status** using `sigil_check_disk`:
   - [ ] Disk is detected (`detected: true`)
   - [ ] Disk is valid (`is_valid: true`)
   - [ ] Presignatures available (`presigs_remaining > 0`)
   - [ ] Not expiring soon (`days_until_expiry > 1`)

2. **Check supported schemes** using `sigil_list_schemes`:
   - Note which scheme the current disk supports
   - Verify it matches your target blockchain

3. **Get signing address** using `sigil_get_address`:
   - Confirm you're using the expected address
   - Verify the address has sufficient balance for your transaction

## Readiness Indicators

### Ready to Sign
- Disk detected and valid
- At least 1 presignature remaining
- Disk not expired
- Correct scheme for target chain

### Warnings
- ⚠️ Less than 100 presignatures remaining
- ⚠️ Disk expires in less than 7 days
- ⚠️ Consider generating a new disk soon

### Not Ready
- ❌ No disk detected
- ❌ Disk invalid or expired
- ❌ No presignatures remaining
- ❌ Scheme mismatch for target chain

## Quick Commands

```
1. sigil_check_disk → Get overall status
2. sigil_get_presig_count → Check remaining presigs
3. sigil_list_schemes → See supported schemes
4. sigil_get_address → Get signing address
```"#;

    Ok(PromptsGetResult {
        description: Some("Check if the system is ready for signing".to_string()),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: PromptContent::Text {
                text: prompt_text.to_string(),
            },
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_prompts() {
        let prompts = get_all_prompts();
        assert!(!prompts.is_empty());
        assert!(prompts.iter().any(|p| p.name == "sign_evm_transfer"));
    }

    #[test]
    fn test_get_sign_evm_transfer_prompt() {
        let mut args = HashMap::new();
        args.insert(
            "to_address".to_string(),
            serde_json::json!("0x742d35Cc6634C0532925a3b844Bc9e7595f12345"),
        );
        args.insert("amount".to_string(), serde_json::json!("0.1"));
        args.insert("chain_id".to_string(), serde_json::json!(1));

        let result = get_prompt("sign_evm_transfer", Some(&args)).unwrap();
        assert!(!result.messages.is_empty());
    }

    #[test]
    fn test_get_troubleshoot_prompt() {
        let result = get_prompt("troubleshoot_disk", None).unwrap();
        assert!(!result.messages.is_empty());
    }
}
