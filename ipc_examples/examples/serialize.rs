use kxkdb::ipc::*;
use kxkdb::qattribute;

#[tokio::main]
async fn main() -> Result<()> {
    let x = K::new_long_list(vec![1,2,3,4,5], qattribute::SORTED);
    println!("x: {}", x);
    let y = x.q_ipc_encode();
    println!("y: {:?}", y);
    let z = K::q_ipc_decode(&y, 1_u8).await;
    println!("z: {}", z);
    Ok(())
}
