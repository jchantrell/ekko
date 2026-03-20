use anyhow::Result;

use super::client;

pub async fn run(uuid: String) -> Result<()> {
    let mut client = client::connect().await?;
    let fact = client.get_edge(&uuid).await?;

    println!("UUID:       {}", fact.uuid);
    println!("Name:       {}", fact.name);
    println!("Fact:       {}", fact.fact);
    if let Some(ref valid) = fact.valid_at {
        println!("Valid at:   {valid}");
    }
    if let Some(ref invalid) = fact.invalid_at {
        println!("Invalid at: {invalid}");
    }
    if let Some(ref created) = fact.created_at {
        println!("Created:    {created}");
    }
    if let Some(ref expired) = fact.expired_at {
        println!("Expired:    {expired}");
    }

    Ok(())
}
