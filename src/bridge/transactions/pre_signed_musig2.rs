use bitcoin::{taproot::TaprootSpendInfo, PublicKey, TapSighashType};
use musig2::{BinaryEncoding, PartialSignature, PubNonce, SecNonce};
use std::collections::HashMap;

use super::{
    super::contexts::{base::BaseContext, verifier::VerifierContext},
    pre_signed::PreSignedTransaction,
    signing::push_taproot_leaf_script_and_control_block_to_witness,
    signing_musig2::{
        generate_nonce, get_aggregated_nonce, get_aggregated_signature, get_partial_signature,
    },
};

pub trait PreSignedMusig2Transaction {
    fn musig2_nonces(&mut self) -> &mut HashMap<usize, HashMap<PublicKey, PubNonce>>;
    fn musig2_signatures(&mut self) -> &mut HashMap<usize, HashMap<PublicKey, PartialSignature>>;
}

pub fn push_nonce<T: PreSignedTransaction + PreSignedMusig2Transaction>(
    tx: &mut T,
    context: &VerifierContext,
    input_index: usize,
) -> SecNonce {
    let musig2_nonces = tx.musig2_nonces();

    let secret_nonce = generate_nonce();
    if musig2_nonces.get(&input_index).is_none() {
        musig2_nonces.insert(input_index, HashMap::new());
    }
    musig2_nonces
        .get_mut(&input_index)
        .unwrap()
        .insert(context.verifier_public_key, secret_nonce.public_nonce());

    secret_nonce
}

pub fn pre_sign_musig2_taproot_input<T: PreSignedTransaction + PreSignedMusig2Transaction>(
    tx: &mut T,
    context: &VerifierContext,
    input_index: usize,
    sighash_type: TapSighashType,
    secret_nonce: &SecNonce,
) {
    // TODO validate nonces first

    let prev_outs = &tx.prev_outs().clone();
    let script = &tx.prev_scripts()[input_index].clone();
    let musig2_nonces = &tx.musig2_nonces()[&input_index]
        .values()
        .map(|public_nonce| public_nonce.clone())
        .collect();

    let partial_signature = get_partial_signature(
        context,
        tx.tx_mut(),
        secret_nonce,
        &get_aggregated_nonce(musig2_nonces),
        input_index,
        prev_outs,
        script,
        sighash_type,
    )
    .unwrap(); // TODO: Add error handling.

    let musig2_signatures = tx.musig2_signatures();
    if musig2_signatures.get(&input_index).is_none() {
        musig2_signatures.insert(input_index, HashMap::new());
    }
    musig2_signatures
        .get_mut(&input_index)
        .unwrap()
        .insert(context.verifier_public_key, partial_signature);
}

pub fn finalize_musig2_taproot_input<T: PreSignedTransaction + PreSignedMusig2Transaction>(
    tx: &mut T,
    context: &dyn BaseContext,
    input_index: usize,
    sighash_type: TapSighashType,
    taproot_spend_info: TaprootSpendInfo,
) {
    // TODO: Verify we have partial signatures from all verifiers.
    // TODO: Verify each signature against the signers public key.
    // See example here: https://github.com/conduition/musig2/blob/c39bfce58098d337a3ec38b54d93def8306d9953/src/signing.rs#L358C1-L366C65

    let prev_outs = &tx.prev_outs().clone();
    let script = &tx.prev_scripts()[input_index].clone();
    let musig2_nonces = &tx.musig2_nonces()[&input_index]
        .values()
        .map(|public_nonce| public_nonce.clone())
        .collect();
    let musig2_signatures = tx.musig2_signatures()[&input_index]
        .values()
        .map(|&partial_signature| PartialSignature::from(partial_signature))
        .collect();
    let tx_mut = tx.tx_mut();

    // Aggregate signature
    let final_signature = get_aggregated_signature(
        context,
        tx_mut,
        &get_aggregated_nonce(musig2_nonces),
        input_index,
        prev_outs,
        script,
        sighash_type,
        musig2_signatures, // TODO: Is there a more elegant way of doing this?
    )
    .unwrap(); // TODO: Add error handling.

    // Push signature to witness
    tx_mut.input[input_index]
        .witness
        .push(final_signature.to_bytes());

    // Push script + control block
    push_taproot_leaf_script_and_control_block_to_witness(
        tx_mut,
        input_index,
        &taproot_spend_info,
        script,
    );
}
