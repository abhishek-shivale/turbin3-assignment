use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account(discriminator = 1)] // anchor add 1byets of vaule (at start of the account data)
pub struct Escrow {
    pub seed: u64, // this allows user to create mutiple ecrow 
    pub maker: Pubkey, // this is user himeself
    pub mint_a: Pubkey, // toke a is what user is offering 
    pub mint_b: Pubkey, // token a user wants in exchange of token a
    pub receive: u64, // how much of mint_b the maker expects
    pub bump: u8, // to able use this for cpi
}