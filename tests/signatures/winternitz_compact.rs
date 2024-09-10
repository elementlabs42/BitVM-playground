use bitcoin_script::script;
use bitvm::{execute_script, signatures::winternitz_compact::{digits_to_number, checksig_verify, sign}};


// TODO: uncomment after u32 winternitz refactor is done
#[tokio::test]
async fn test_winternitz_compact_u32_signature_success() {
  // let secret_key = "3076ca1dfc1e383be26d5dd3c0c427340f96139fa8c2520862cf551ec2d670ac"; // TODO replace with secret key for specific variable, generate and store secrets in local client
  // let block: u32 = 860033;
  // // 0000 0000 0000 1101 0001 1111 1000 0001
  // let message = [0, 0, 0, 13, 1, 15, 8, 1];

  // let signature = sign(&secret_key, message);
  // let locking_script = checksig_verify(secret_key);
  // println!("signature: {:?}", sign(&secret_key, message).compile());
  // println!("locking_script: {:?}", checksig_verify(secret_key).compile());
  // let script = script!{
  //   { signature }
  //   { locking_script }
  //   { digits_to_number() }
  //   { block }
  //   OP_EQUAL
  // };
  // let exec_result = execute_script(script);

  // println!("last_opcode: {:?}", exec_result.last_opcode);
  // println!("remaining_script: {:?}", exec_result.remaining_script);
  // println!("error: {:?}", exec_result.error);
  // println!("success: {:?}", exec_result.success);
  // println!("final_stack: {}", exec_result.final_stack);
  // assert!(exec_result.success);
}
