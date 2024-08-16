use bitcoin::{
    hex::{Case::Upper, DisplayHex},
    key::Keypair,
    Amount, Network, OutPoint, PublicKey, ScriptBuf, Txid, XOnlyPublicKey,
};
use esplora_client::{AsyncClient, Error, TxStatus};
use musig2::SecNonce;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
};

use super::{
    super::{
        constants::{NUM_BLOCKS_PER_2_WEEKS, NUM_BLOCKS_PER_4_WEEKS},
        contexts::{base::BaseContext, operator::OperatorContext, verifier::VerifierContext},
        transactions::{
            assert::AssertTransaction,
            base::{validate_transaction, BaseTransaction, Input, InputWithScript},
            burn::BurnTransaction,
            challenge::ChallengeTransaction,
            disprove::DisproveTransaction,
            kick_off_1::KickOffTransaction,
            peg_out::PegOutTransaction,
            pre_signed::PreSignedTransaction,
            take_1::Take1Transaction,
            take_2::Take2Transaction,
        },
    },
    base::{get_block_height, verify_if_not_mined, verify_tx_result, BaseGraph, GRAPH_VERSION},
    peg_in::PegInGraph,
};

pub enum PegOutDepositorStatus {
    PegOutNotStarted, // peg-out transaction not created yet
    PegOutWait,       // peg-out not confirmed yet, wait
    PegOutComplete,   // peg-out complete
}

impl Display for PegOutDepositorStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            PegOutDepositorStatus::PegOutNotStarted => {
                write!(f, "Peg-out available. Request peg-out?")
            }
            PegOutDepositorStatus::PegOutWait => write!(f, "No action available. Wait..."),
            PegOutDepositorStatus::PegOutComplete => write!(f, "Peg-out complete. Done."),
        }
    }
}

pub enum PegOutVerifierStatus {
    PegOutPresign,           // should presign peg-out graph
    PegOutComplete,          // peg-out complete
    PegOutWait,              // no action required, wait
    PegOutChallengeAvailabe, // can challenge
    PegOutBurnAvailable,
    PegOutDisproveAvailable,
    PegOutFailed, // burn or disprove executed
}

impl Display for PegOutVerifierStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            PegOutVerifierStatus::PegOutPresign => {
                write!(f, "Signatures required. Presign peg-out transactions?")
            }
            PegOutVerifierStatus::PegOutComplete => {
                write!(f, "Peg-out complete, reimbursement succeded. Done.")
            }
            PegOutVerifierStatus::PegOutWait => write!(f, "No action available. Wait..."),
            PegOutVerifierStatus::PegOutChallengeAvailabe => {
                write!(
                    f,
                    "Kick-off transaction confirmed, dispute available. Broadcast challenge transaction?"
                )
            }
            PegOutVerifierStatus::PegOutBurnAvailable => {
                write!(f, "Kick-off timed out. Broadcast burn transaction?")
            }
            PegOutVerifierStatus::PegOutDisproveAvailable => {
                write!(
                    f,
                    "Assert transaction confirmed. Broadcast disprove transaction?"
                )
            }
            PegOutVerifierStatus::PegOutFailed => {
                write!(f, "Peg-out complete, reimbursement failed. Done.")
            }
        }
    }
}

pub enum PegOutOperatorStatus {
    PegOutWait,
    PegOutComplete,    // peg-out complete
    PegOutFailed,      // burn or disprove executed
    PegOutStartPegOut, // should execute peg-out tx
    PegOutKickOffAvailable,
    PegOutAssertAvailable,
    PegOutTake1Available,
    PegOutTake2Available,
}

impl Display for PegOutOperatorStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            PegOutOperatorStatus::PegOutWait => write!(f, "No action available. Wait..."),
            PegOutOperatorStatus::PegOutComplete => {
                write!(f, "Peg-out complete, reimbursement succeded. Done.")
            }
            PegOutOperatorStatus::PegOutFailed => {
                write!(f, "Peg-out complete, reimbursement failed. Done.")
            }
            PegOutOperatorStatus::PegOutStartPegOut => {
                write!(f, "Peg-out requested. Broadcast peg-out transaction?")
            }
            PegOutOperatorStatus::PegOutKickOffAvailable => {
                write!(f, "Peg-out confirmed. Broadcast kick-off transaction?")
            }
            PegOutOperatorStatus::PegOutAssertAvailable => {
                write!(f, "Dispute raised. Broadcast assert transaction?")
            }
            PegOutOperatorStatus::PegOutTake1Available => write!(
                f,
                "Dispute timed out, reimbursement available. Broadcast take 1 transaction?"
            ),
            PegOutOperatorStatus::PegOutTake2Available => write!(
                f,
                "Dispute timed out, reimbursement available. Broadcast take 2 transaction?"
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct PegOutGraph {
    version: String,
    network: Network,
    id: String,

    // state: State,
    // n_of_n_pre_signing_state: PreSigningState,
    n_of_n_presigned: bool,
    n_of_n_public_key: PublicKey,
    n_of_n_taproot_public_key: XOnlyPublicKey,

    pub peg_in_graph_id: String,
    peg_in_confirm_txid: Txid,
    kick_off_transaction: KickOffTransaction,
    take_1_transaction: Take1Transaction,
    challenge_transaction: ChallengeTransaction,
    assert_transaction: AssertTransaction,
    take_2_transaction: Take2Transaction,
    disprove_transaction: DisproveTransaction,
    burn_transaction: BurnTransaction,

    operator_public_key: PublicKey,
    operator_taproot_public_key: XOnlyPublicKey,

    withdrawer_public_key: Option<PublicKey>,
    withdrawer_taproot_public_key: Option<XOnlyPublicKey>,
    withdrawer_evm_address: Option<String>,

    peg_out_transaction: Option<PegOutTransaction>,
}

impl BaseGraph for PegOutGraph {
    fn network(&self) -> Network { self.network }

    fn id(&self) -> &String { &self.id }
}

impl PegOutGraph {
    pub fn new(context: &OperatorContext, peg_in_graph: &PegInGraph, kickoff_input: Input) -> Self {
        let kick_off_transaction = KickOffTransaction::new(context, kickoff_input);
        let kick_off_txid = kick_off_transaction.tx().compute_txid();

        let peg_in_confirm_transaction = peg_in_graph.peg_in_confirm_transaction_ref();
        let peg_in_confirm_txid = peg_in_confirm_transaction.tx().compute_txid();
        let take_1_vout_0 = 0;
        let take_1_vout_1 = 0;
        let take_1_vout_2 = 1;
        let take_1_vout_3 = 2;
        let take_1_transaction = Take1Transaction::new(
            context,
            Input {
                outpoint: OutPoint {
                    txid: peg_in_confirm_txid,
                    vout: take_1_vout_0.to_u32().unwrap(),
                },
                amount: peg_in_confirm_transaction.tx().output[take_1_vout_0].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: take_1_vout_1.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[take_1_vout_1].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: take_1_vout_2.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[take_1_vout_2].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: take_1_vout_3.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[take_1_vout_3].value,
            },
        );

        let input_amount_crowdfunding = Amount::from_btc(1.0).unwrap(); // TODO replace placeholder
        let challenge_vout_0 = 1;
        let challenge_transaction = ChallengeTransaction::new(
            context,
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: challenge_vout_0.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[challenge_vout_0].value,
            },
            input_amount_crowdfunding,
        );

        let assert_vout_0 = 2;
        let assert_transaction = AssertTransaction::new(
            context,
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: assert_vout_0.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[assert_vout_0].value,
            },
        );
        let assert_txid = kick_off_transaction.tx().compute_txid();

        let take_2_vout_0 = 0;
        let take_2_vout_1 = 0;
        let take_2_vout_2 = 1;
        let take_2_transaction = Take2Transaction::new(
            context,
            Input {
                outpoint: OutPoint {
                    txid: peg_in_confirm_txid,
                    vout: take_2_vout_0.to_u32().unwrap(),
                },
                amount: peg_in_confirm_transaction.tx().output[take_2_vout_0].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: take_2_vout_1.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[take_2_vout_1].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: take_2_vout_2.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[take_2_vout_2].value,
            },
        );

        let script_index = 1; // TODO replace placeholder
        let disprove_vout_0 = 1;
        let disprove_vout_1 = 2;
        let disprove_transaction = DisproveTransaction::new(
            context,
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: disprove_vout_0.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[disprove_vout_0].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: disprove_vout_1.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[disprove_vout_1].value,
            },
            script_index,
        );

        let burn_vout_0 = 2;
        let burn_transaction = BurnTransaction::new(
            context,
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: burn_vout_0.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[burn_vout_0].value,
            },
        );

        PegOutGraph {
            version: GRAPH_VERSION.to_string(),
            network: context.network,
            id: generate_id(peg_in_graph, &context.operator_public_key),
            n_of_n_presigned: false,
            n_of_n_public_key: context.n_of_n_public_key,
            n_of_n_taproot_public_key: context.n_of_n_taproot_public_key,
            peg_in_graph_id: peg_in_graph.id().clone(),
            peg_in_confirm_txid,
            kick_off_transaction,
            take_1_transaction,
            challenge_transaction,
            assert_transaction,
            take_2_transaction,
            disprove_transaction,
            burn_transaction,
            operator_public_key: context.operator_public_key,
            operator_taproot_public_key: context.operator_taproot_public_key,
            withdrawer_public_key: None,
            withdrawer_taproot_public_key: None,
            withdrawer_evm_address: None,
            peg_out_transaction: None,
        }
    }

    pub fn new_for_validation(&self) -> Self {
        let kick_off_transaction = KickOffTransaction::new_for_validation(
            self.network,
            &self.operator_public_key,
            &self.operator_taproot_public_key,
            &self.n_of_n_taproot_public_key,
            Input {
                outpoint: self.kick_off_transaction.tx().input[0].previous_output,
                amount: self.kick_off_transaction.prev_outs()[0].value,
            },
        );
        let kick_off_txid = kick_off_transaction.tx().compute_txid();

        // let peg_in_confirm_transaction = peg_in_graph.peg_in_confirm_transaction_ref();
        // let peg_in_confirm_txid = peg_in_confirm_transaction.tx().compute_txid();
        let peg_in_confirm_txid = self.take_1_transaction.tx().input[0].previous_output.txid; // Self-referencing
        let take_1_vout_0 = 0;
        let take_1_vout_1 = 0;
        let take_1_vout_2 = 1;
        let take_1_vout_3 = 2;
        let take_1_transaction = Take1Transaction::new_for_validation(
            self.network,
            &self.operator_public_key,
            &self.operator_taproot_public_key,
            &self.n_of_n_taproot_public_key,
            Input {
                outpoint: OutPoint {
                    txid: peg_in_confirm_txid,
                    vout: take_1_vout_0.to_u32().unwrap(),
                },
                amount: self.take_1_transaction.prev_outs()[0].value, // Self-referencing
            },
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: take_1_vout_1.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[take_1_vout_1].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: take_1_vout_2.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[take_1_vout_2].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: take_1_vout_3.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[take_1_vout_3].value,
            },
        );

        let input_amount_crowdfunding = Amount::from_btc(1.0).unwrap(); // TODO replace placeholder
        let challenge_vout_0 = 1;
        let challenge_transaction = ChallengeTransaction::new_for_validation(
            self.network,
            &self.operator_public_key,
            &self.operator_taproot_public_key,
            &self.n_of_n_taproot_public_key,
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: challenge_vout_0.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[challenge_vout_0].value,
            },
            input_amount_crowdfunding,
        );

        let assert_vout_0 = 2;
        let assert_transaction = AssertTransaction::new_for_validation(
            self.network,
            &self.operator_public_key,
            &self.n_of_n_taproot_public_key,
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: assert_vout_0.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[assert_vout_0].value,
            },
        );
        let assert_txid = kick_off_transaction.tx().compute_txid();

        let take_2_vout_0 = 0;
        let take_2_vout_1 = 0;
        let take_2_vout_2 = 1;
        let take_2_transaction = Take2Transaction::new_for_validation(
            self.network,
            &self.operator_public_key,
            &self.n_of_n_taproot_public_key,
            Input {
                outpoint: OutPoint {
                    txid: peg_in_confirm_txid,
                    vout: take_2_vout_0.to_u32().unwrap(),
                },
                amount: self.take_2_transaction.prev_outs()[0].value, // Self-referencing
            },
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: take_2_vout_1.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[take_2_vout_1].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: take_2_vout_2.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[take_2_vout_2].value,
            },
        );

        let script_index = 1; // TODO replace placeholder
        let disprove_vout_0 = 1;
        let disprove_vout_1 = 2;
        let disprove_transaction = DisproveTransaction::new_for_validation(
            self.network,
            &self.n_of_n_taproot_public_key,
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: disprove_vout_0.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[disprove_vout_0].value,
            },
            Input {
                outpoint: OutPoint {
                    txid: assert_txid,
                    vout: disprove_vout_1.to_u32().unwrap(),
                },
                amount: assert_transaction.tx().output[disprove_vout_1].value,
            },
            script_index,
        );

        let burn_vout_0 = 2;
        let burn_transaction = BurnTransaction::new_for_validation(
            self.network,
            &self.n_of_n_taproot_public_key,
            Input {
                outpoint: OutPoint {
                    txid: kick_off_txid,
                    vout: burn_vout_0.to_u32().unwrap(),
                },
                amount: kick_off_transaction.tx().output[burn_vout_0].value,
            },
        );

        PegOutGraph {
            version: GRAPH_VERSION.to_string(),
            network: self.network,
            id: self.id.clone(),
            n_of_n_presigned: false,
            n_of_n_public_key: self.n_of_n_public_key,
            n_of_n_taproot_public_key: self.n_of_n_taproot_public_key,
            peg_in_graph_id: self.peg_in_graph_id.clone(),
            peg_in_confirm_txid,
            kick_off_transaction,
            take_1_transaction,
            challenge_transaction,
            assert_transaction,
            take_2_transaction,
            disprove_transaction,
            burn_transaction,
            operator_public_key: self.operator_public_key,
            operator_taproot_public_key: self.operator_taproot_public_key,
            withdrawer_public_key: None,
            withdrawer_taproot_public_key: None,
            withdrawer_evm_address: None,
            peg_out_transaction: None,
        }
    }

    pub fn push_nonces(
        &mut self,
        context: &VerifierContext,
    ) -> HashMap<Txid, HashMap<usize, SecNonce>> {
        let mut secret_nonces = HashMap::new();

        secret_nonces.insert(
            self.take_1_transaction.tx().compute_txid(),
            self.take_1_transaction.push_nonces(context),
        );
        secret_nonces.insert(
            self.assert_transaction.tx().compute_txid(),
            self.assert_transaction.push_nonces(context),
        );
        secret_nonces.insert(
            self.take_2_transaction.tx().compute_txid(),
            self.take_2_transaction.push_nonces(context),
        );
        secret_nonces.insert(
            self.disprove_transaction.tx().compute_txid(),
            self.disprove_transaction.push_nonces(context),
        );
        secret_nonces.insert(
            self.burn_transaction.tx().compute_txid(),
            self.burn_transaction.push_nonces(context),
        );

        secret_nonces
    }

    pub fn pre_sign(
        &mut self,
        context: &VerifierContext,
        secret_nonces: &HashMap<Txid, HashMap<usize, SecNonce>>,
    ) {
        self.assert_transaction.pre_sign(
            context,
            &secret_nonces[&self.assert_transaction.tx().compute_txid()],
        );
        self.burn_transaction.pre_sign(
            context,
            &secret_nonces[&self.burn_transaction.tx().compute_txid()],
        );
        self.disprove_transaction.pre_sign(
            context,
            &secret_nonces[&self.disprove_transaction.tx().compute_txid()],
        );
        self.take_1_transaction.pre_sign(
            context,
            &secret_nonces[&self.take_1_transaction.tx().compute_txid()],
        );
        self.take_2_transaction.pre_sign(
            context,
            &secret_nonces[&self.take_2_transaction.tx().compute_txid()],
        );

        self.n_of_n_presigned = true; // TODO: set to true after collecting all n of n signatures
    }

    pub async fn verifier_status(&self, client: &AsyncClient) -> PegOutVerifierStatus {
        if self.n_of_n_presigned {
            let (
                kick_off_status,
                challenge_status,
                assert_status,
                disprove_status,
                burn_status,
                take_1_status,
                take_2_status,
                _,
            ) = Self::get_peg_out_statuses(self, client).await;
            let blockchain_height = get_block_height(client).await;

            if kick_off_status
                .as_ref()
                .is_ok_and(|status| status.confirmed)
            {
                // check take 1 and take 2
                if take_1_status.as_ref().is_ok_and(|status| status.confirmed)
                    || take_2_status.as_ref().is_ok_and(|status| status.confirmed)
                {
                    return PegOutVerifierStatus::PegOutComplete;
                }

                // check burn and disprove
                if burn_status.as_ref().is_ok_and(|status| status.confirmed)
                    || disprove_status
                        .as_ref()
                        .is_ok_and(|status| status.confirmed)
                {
                    return PegOutVerifierStatus::PegOutFailed; // TODO: can be also `PegOutVerifierStatus::PegOutComplete`
                }

                if kick_off_status
                    .as_ref()
                    .unwrap()
                    .block_height
                    .is_some_and(|block_height| {
                        block_height + NUM_BLOCKS_PER_4_WEEKS > blockchain_height
                    })
                {
                    if challenge_status
                        .as_ref()
                        .is_ok_and(|status| !status.confirmed)
                    {
                        return PegOutVerifierStatus::PegOutChallengeAvailabe;
                    } else if assert_status.as_ref().is_ok_and(|status| status.confirmed) {
                        return PegOutVerifierStatus::PegOutDisproveAvailable;
                    } else {
                        return PegOutVerifierStatus::PegOutWait;
                    }
                } else {
                    if assert_status.is_ok_and(|status| !status.confirmed) {
                        return PegOutVerifierStatus::PegOutBurnAvailable; // TODO: challange and burn available here
                    } else {
                        return PegOutVerifierStatus::PegOutDisproveAvailable;
                    }
                }
            } else {
                return PegOutVerifierStatus::PegOutWait;
            }
        } else {
            return PegOutVerifierStatus::PegOutPresign;
        }
    }

    pub async fn operator_status(&self, client: &AsyncClient) -> PegOutOperatorStatus {
        if self.n_of_n_presigned {
            let (
                kick_off_status,
                challenge_status,
                assert_status,
                disprove_status,
                burn_status,
                take_1_status,
                take_2_status,
                peg_out_status,
            ) = Self::get_peg_out_statuses(self, client).await;
            let blockchain_height = get_block_height(client).await;

            if peg_out_status.is_some_and(|status| status.unwrap().confirmed) {
                if kick_off_status
                    .as_ref()
                    .is_ok_and(|status| status.confirmed)
                {
                    // check take 1 and take 2
                    if take_1_status.as_ref().is_ok_and(|status| status.confirmed)
                        || take_2_status.as_ref().is_ok_and(|status| status.confirmed)
                    {
                        return PegOutOperatorStatus::PegOutComplete;
                    }

                    // check burn and disprove
                    if burn_status.as_ref().is_ok_and(|status| status.confirmed)
                        || disprove_status
                            .as_ref()
                            .is_ok_and(|status| status.confirmed)
                    {
                        return PegOutOperatorStatus::PegOutFailed; // TODO: can be also `PegOutOperatorStatus::PegOutComplete`
                    }

                    if challenge_status.is_ok_and(|status| status.confirmed) {
                        if assert_status.as_ref().is_ok_and(|status| status.confirmed) {
                            if assert_status.as_ref().unwrap().block_height.is_some_and(
                                |block_height| {
                                    block_height + NUM_BLOCKS_PER_2_WEEKS <= blockchain_height
                                },
                            ) {
                                return PegOutOperatorStatus::PegOutTake2Available;
                            } else {
                                return PegOutOperatorStatus::PegOutWait;
                            }
                        } else {
                            return PegOutOperatorStatus::PegOutAssertAvailable;
                        }
                    } else {
                        if kick_off_status.as_ref().unwrap().block_height.is_some_and(
                            |block_height| {
                                block_height + NUM_BLOCKS_PER_2_WEEKS <= blockchain_height
                            },
                        ) {
                            return PegOutOperatorStatus::PegOutTake1Available;
                        } else {
                            return PegOutOperatorStatus::PegOutWait;
                        }
                    }
                } else {
                    return PegOutOperatorStatus::PegOutKickOffAvailable;
                }
            } else {
                return PegOutOperatorStatus::PegOutStartPegOut;
            }
        } else {
            return PegOutOperatorStatus::PegOutWait;
        }
    }

    pub async fn depositor_status(&self, client: &AsyncClient) -> PegOutDepositorStatus {
        if self.peg_out_transaction.is_some() {
            let peg_out_txid = self
                .peg_out_transaction
                .as_ref()
                .unwrap()
                .tx()
                .compute_txid();
            let peg_out_status = client.get_tx_status(&peg_out_txid).await;

            if peg_out_status.is_ok_and(|status| status.confirmed) {
                return PegOutDepositorStatus::PegOutComplete;
            } else {
                return PegOutDepositorStatus::PegOutWait;
            }
        } else {
            return PegOutDepositorStatus::PegOutNotStarted;
        }
    }

    pub async fn kick_off(&mut self, client: &AsyncClient) {
        verify_if_not_mined(&client, self.kick_off_transaction.tx().compute_txid()).await;

        // complete kick_off tx
        let kick_off_tx = self.kick_off_transaction.finalize();

        // broadcast kick_off tx
        let kick_off_result = client.broadcast(&kick_off_tx).await;

        // verify kick_off tx result
        verify_tx_result(&kick_off_result);
    }

    pub async fn challenge(
        &mut self,
        client: &AsyncClient,
        context: &dyn BaseContext,
        crowdfundng_inputs: &Vec<InputWithScript<'_>>,
        keypair: &Keypair,
        output_script_pubkey: ScriptBuf,
    ) {
        verify_if_not_mined(client, self.challenge_transaction.tx().compute_txid()).await;

        let kick_off_txid = self.kick_off_transaction.tx().compute_txid();
        let kick_off_status = client.get_tx_status(&kick_off_txid).await;

        if kick_off_status.is_ok_and(|status| status.confirmed) {
            // complete challenge tx
            self.challenge_transaction.add_inputs_and_output(
                context,
                crowdfundng_inputs,
                keypair,
                output_script_pubkey,
            );
            let challenge_tx = self.challenge_transaction.finalize();

            // broadcast challenge tx
            let challenge_result = client.broadcast(&challenge_tx).await;

            // verify challenge tx result
            verify_tx_result(&challenge_result);
        } else {
            panic!("Kick-off tx has not been yet confirmed!");
        }
    }

    pub async fn assert(&mut self, client: &AsyncClient) {
        verify_if_not_mined(client, self.assert_transaction.tx().compute_txid()).await;

        let kick_off_txid = self.kick_off_transaction.tx().compute_txid();
        let kick_off_status = client.get_tx_status(&kick_off_txid).await;

        if kick_off_status.is_ok_and(|status| status.confirmed) {
            // complete assert tx
            // TODO: commit ZK computation result
            let assert_tx = self.assert_transaction.finalize();

            // broadcast assert tx
            let assert_result = client.broadcast(&assert_tx).await;

            // verify assert tx result
            verify_tx_result(&assert_result);
        } else {
            panic!("Kick-off tx has not been yet confirmed!");
        }
    }

    pub async fn disprove(
        &mut self,
        client: &AsyncClient,
        input_script_index: u32,
        output_script_pubkey: ScriptBuf,
    ) {
        verify_if_not_mined(client, self.disprove_transaction.tx().compute_txid()).await;

        let assert_txid = self.assert_transaction.tx().compute_txid();
        let assert_status = client.get_tx_status(&assert_txid).await;

        if assert_status.is_ok_and(|status| status.confirmed) {
            // complete disprove tx
            self.disprove_transaction
                .add_input_output(input_script_index, output_script_pubkey);
            let disprove_tx = self.disprove_transaction.finalize();

            // broadcast disprove tx
            let disprove_result = client.broadcast(&disprove_tx).await;

            // verify disprove tx result
            verify_tx_result(&disprove_result);
        } else {
            panic!("Assert tx has not been yet confirmed!");
        }
    }

    pub async fn burn(&mut self, client: &AsyncClient, output_script_pubkey: ScriptBuf) {
        verify_if_not_mined(client, self.burn_transaction.tx().compute_txid()).await;

        let kick_off_txid = self.kick_off_transaction.tx().compute_txid();
        let kick_off_status = client.get_tx_status(&kick_off_txid).await;

        let blockchain_height = get_block_height(client).await;

        if kick_off_status
            .as_ref()
            .is_ok_and(|status| status.confirmed)
        {
            if kick_off_status
                .as_ref()
                .unwrap()
                .block_height
                .is_some_and(|block_height| {
                    block_height + NUM_BLOCKS_PER_4_WEEKS <= blockchain_height
                })
            {
                // complete burn tx
                self.burn_transaction.add_output(output_script_pubkey);
                let burn_tx = self.burn_transaction.finalize();

                // broadcast burn tx
                let burn_result = client.broadcast(&burn_tx).await;

                // verify burn tx result
                verify_tx_result(&burn_result);
            } else {
                panic!("Kick-off timelock has not yet elapsed!");
            }
        } else {
            panic!("Kick-off tx has not been yet confirmed!");
        }
    }

    pub async fn take_1(&mut self, client: &AsyncClient) {
        verify_if_not_mined(&client, self.take_1_transaction.tx().compute_txid()).await;
        verify_if_not_mined(&client, self.challenge_transaction.tx().compute_txid()).await;
        verify_if_not_mined(&client, self.assert_transaction.tx().compute_txid()).await;
        verify_if_not_mined(&client, self.burn_transaction.tx().compute_txid()).await;

        let peg_in_confirm_status = client.get_tx_status(&self.peg_in_confirm_txid).await;
        let kick_off_txid = self.kick_off_transaction.tx().compute_txid();
        let kick_off_status = client.get_tx_status(&kick_off_txid).await;

        let blockchain_height = get_block_height(client).await;

        if peg_in_confirm_status.is_ok_and(|status| status.confirmed)
            && kick_off_status
                .as_ref()
                .is_ok_and(|status| status.confirmed)
        {
            if kick_off_status
                .unwrap()
                .block_height
                .is_some_and(|block_height| {
                    block_height + NUM_BLOCKS_PER_2_WEEKS <= blockchain_height
                })
            {
                // complete take 1 tx
                let take_1_tx = self.take_1_transaction.finalize();

                // broadcast take 1 tx
                let take_1_result = client.broadcast(&take_1_tx).await;

                // verify take 1 tx result
                verify_tx_result(&take_1_result);
            } else {
                panic!("Kick-off tx timelock has not yet elapsed!");
            }
        } else {
            panic!("Neither peg-in confirm tx nor kick-off tx has not been yet confirmed!");
        }
    }

    pub async fn take_2(&mut self, client: &AsyncClient) {
        verify_if_not_mined(&client, self.take_2_transaction.tx().compute_txid()).await;
        verify_if_not_mined(&client, self.take_1_transaction.tx().compute_txid()).await;
        verify_if_not_mined(&client, self.disprove_transaction.tx().compute_txid()).await;
        verify_if_not_mined(&client, self.burn_transaction.tx().compute_txid()).await;

        let peg_in_confirm_status = client.get_tx_status(&self.peg_in_confirm_txid).await;
        let assert_txid = self.assert_transaction.tx().compute_txid();
        let assert_status = client.get_tx_status(&assert_txid).await;

        let blockchain_height = get_block_height(client).await;

        if peg_in_confirm_status.is_ok_and(|status| status.confirmed)
            && assert_status.as_ref().is_ok_and(|status| status.confirmed)
        {
            if assert_status
                .unwrap()
                .block_height
                .is_some_and(|block_height| {
                    block_height + NUM_BLOCKS_PER_2_WEEKS <= blockchain_height
                })
            {
                // complete take 2 tx
                let take_2_tx = self.take_2_transaction.finalize();

                // broadcast take 2 tx
                let take_2_result = client.broadcast(&take_2_tx).await;

                // verify take 2 tx result
                verify_tx_result(&take_2_result);
            } else {
                panic!("Assert tx timelock has not yet elapsed!");
            }
        } else {
            panic!("Neither peg-in confirm tx nor assert tx has not been yet confirmed!");
        }
    }

    async fn get_peg_out_statuses(
        &self,
        client: &AsyncClient,
    ) -> (
        Result<TxStatus, Error>,
        Result<TxStatus, Error>,
        Result<TxStatus, Error>,
        Result<TxStatus, Error>,
        Result<TxStatus, Error>,
        Result<TxStatus, Error>,
        Result<TxStatus, Error>,
        Option<Result<TxStatus, Error>>,
    ) {
        let kick_off_status = client
            .get_tx_status(&self.kick_off_transaction.tx().compute_txid())
            .await;
        let challenge_status = client
            .get_tx_status(&self.challenge_transaction.tx().compute_txid())
            .await;
        let assert_status = client
            .get_tx_status(&self.assert_transaction.tx().compute_txid())
            .await;
        let disprove_status = client
            .get_tx_status(&self.disprove_transaction.tx().compute_txid())
            .await;
        let burn_status = client
            .get_tx_status(&self.burn_transaction.tx().compute_txid())
            .await;
        let take_1_status = client
            .get_tx_status(&self.take_1_transaction.tx().compute_txid())
            .await;
        let take_2_status = client
            .get_tx_status(&self.take_2_transaction.tx().compute_txid())
            .await;

        let mut peg_out_status: Option<Result<TxStatus, Error>> = None;
        if self.peg_out_transaction.is_some() {
            peg_out_status = Some(
                client
                    .get_tx_status(&self.take_2_transaction.tx().compute_txid())
                    .await,
            );
        }

        return (
            kick_off_status,
            challenge_status,
            assert_status,
            disprove_status,
            burn_status,
            take_1_status,
            take_2_status,
            peg_out_status,
        );
    }

    pub fn validate(&self) -> bool {
        // kick_off_transaction: KickOffTransaction,
        // take_1_transaction: Take1Transaction,
        // challenge_transaction: ChallengeTransaction,
        // assert_transaction: AssertTransaction,
        // take_2_transaction: Take2Transaction,
        // disprove_transaction: DisproveTransaction,
        // burn_transaction: BurnTransaction,
        // peg_out_transaction

        let peg_out_graph = self.new_for_validation();
        if !validate_transaction(
            self.kick_off_transaction.tx(),
            peg_out_graph.kick_off_transaction.tx(),
        ) || !validate_transaction(
            self.take_1_transaction.tx(),
            peg_out_graph.take_1_transaction.tx(),
        ) || !validate_transaction(
            self.challenge_transaction.tx(),
            peg_out_graph.challenge_transaction.tx(),
        ) || !validate_transaction(
            self.assert_transaction.tx(),
            peg_out_graph.assert_transaction.tx(),
        ) || !validate_transaction(
            self.take_2_transaction.tx(),
            peg_out_graph.take_2_transaction.tx(),
        ) || !validate_transaction(
            self.disprove_transaction.tx(),
            peg_out_graph.disprove_transaction.tx(),
        ) || !validate_transaction(
            self.burn_transaction.tx(),
            peg_out_graph.burn_transaction.tx(),
        ) {
            return false;
        }

        true
    }

    pub fn merge(&mut self, source_peg_out_graph: &PegOutGraph) {
        self.challenge_transaction
            .merge(&source_peg_out_graph.challenge_transaction);
        self.assert_transaction
            .merge(&source_peg_out_graph.assert_transaction);
        self.disprove_transaction
            .merge(&source_peg_out_graph.disprove_transaction);
        self.burn_transaction
            .merge(&source_peg_out_graph.burn_transaction);
        self.take_1_transaction
            .merge(&source_peg_out_graph.take_1_transaction);
        self.take_2_transaction
            .merge(&source_peg_out_graph.take_2_transaction);
    }
}

pub fn generate_id(peg_in_graph: &PegInGraph, operator_public_key: &PublicKey) -> String {
    let mut hasher = Sha256::new();

    hasher.update(peg_in_graph.id().to_string() + &operator_public_key.to_string());

    hasher.finalize().to_hex_string(Upper)
}
