use anchor_lang::{prelude::*, solana_program};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Transfer},
};

use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::{
    clock::MAX_RECENT_BLOCKHASHES, secp256k1_recover::secp256k1_recover,
};
use anchor_lang::Accounts;
use std::convert::TryInto;
use std::{collections::HashMap, mem};

declare_id!("DhzZ4J9xF7Q9ECsyuC8NL1Zy1XToLSkpUfhE7k57rYP1");
mod error;

#[program]
pub mod gacha_marketplace {
    use super::*;

    pub fn init_state(ctx: Context<InitState>, _listing_price: u128) -> Result<()> {
        let program_state = &mut ctx.accounts.state_account;
        if program_state.initialized == true {
            return Err(error::Error::StateAlreadyInitialized.into());
        }
        let empty_map: Vec<MarketItem> = Vec::new();

        program_state.map = empty_map;
        program_state.item_ids = 0;
        program_state.item_sold = 0;
        program_state.owner = ctx.accounts.user.key();
        program_state.initialized = true;
        program_state.listing_price = _listing_price;

        Ok(())
    }

    pub fn create_market_item(
        ctx: Context<CreateMarketItem>,
        // _token_program_id: Pubkey, // program id,
        // _mint_address: Pubkey,     // ATA
        _price: u128,
        // _file_name: String,
        // _description: String,
        // _cash_back: u8,
    ) -> Result<()> {
        require!(_price > 0, error::Error::InvalidPrice);
        // require!(_cash_back < 100, error::Error::CashbackMax);

        let state = &mut ctx.accounts.state_account;

        let item = MarketItem {
            item_id: state.item_ids,
            // token_program_id: _token_program_id,
            // mint_address: _mint_address,
            // seller: ctx.accounts.user.key(),
            owner: None,
            price: _price,
            // file_name: _file_name,
            // description: _description,
            // cash_back: _cash_back,
            sold: false,
            gacha: false,
        };
        state.item_ids += 1;
        state.map.push(item);

        // TRANSFER NFT //
        let transfer_instruction = Transfer {
            from: ctx.accounts.from.to_account_info(),
            to: ctx.accounts.to.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, transfer_instruction);
        anchor_spl::token::transfer(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn purchase_sale(
        ctx: Context<Purchase>,
        _price: u128,
        _item_id: u128,
        _bump: u8,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state_account;

        let mut item = state.map.get(_item_id as usize).unwrap().to_owned();
        require!(_price == item.price, error::Error::InvalidPayment);

        // transfer listing price to owner
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.user.key(),
                &ctx.accounts.seller.key(),
                _price.try_into().unwrap(),
            ),
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.seller.to_account_info(),
            ],
        )?;

        // transfer nft
        let seeds = vec![_bump];
        let seeds = vec![b"auth".as_ref(), seeds.as_slice()];
        let seeds = vec![seeds.as_slice()];
        let seeds = seeds.as_slice();
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.from_token_account.to_account_info(),
                to: ctx.accounts.to_token_account.to_account_info(),
                authority: ctx.accounts.auth.to_account_info(),
            },
            &seeds,
        );
        anchor_spl::token::transfer(cpi_ctx, 1)?;

        item.owner = Some(ctx.accounts.user.key());
        item.sold = true;
        state.item_sold += 1;

        Ok(())
    }

    pub fn gacha<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, Gacha<'info>>,
        _qty: u8,
        _price: u128,
        _fee: u128,
        _bump: u8,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state_account;
        let remaining_accounts = &mut ctx.remaining_accounts;

        let item_count = state.item_ids;
        let item_sold = state.item_sold;
        let unsold_item_count = item_count - item_sold;

        let mut items = Vec::new();
        for i in 0..item_count {
            let item = state.map.get(i as usize).unwrap().to_owned();
            if item.owner == None && item.price == _price {
                items.insert(i as usize, item);
            }
        }

        let mut gacha_items = HashMap::new();
        require!(items.len() > 0, error::Error::ItemsUnavailableForGacha);
        require!(
            usize::from(_qty) <= items.len(),
            error::Error::InvalidQuantity
        );
        let mut random_output: Vec<u64> = Vec::new();
        let mut seed = Clock::get()?.unix_timestamp as u64;
        let mut gacha_index = 0;
        for i in 0.._qty {
            seed ^= seed >> 12;
            seed ^= seed << 25;
            seed ^= seed >> 27;
            // seed *= 0x2545F4914F6CDD1D;
            loop {
                gacha_index = ((generate_random_f64(seed) * 1000.0) as u64) % (items.len() as u64);
                if random_output.contains(&(gacha_index)) == false {
                    break;
                }
                seed += 16454654645667;
            }

            let item = items.get(*&gacha_index as usize).unwrap().to_owned();
            gacha_items.insert(i, item);
            random_output.push(gacha_index);
        }

        let seeds = vec![_bump];
        let seeds = vec![b"auth".as_ref(), seeds.as_slice()];
        let seeds = vec![seeds.as_slice()];
        let seeds = seeds.as_slice();
        for idx in 0..(_qty as usize) {
            let item_id = gacha_items.get(&(idx as u8)).unwrap().item_id;
            let random = *&random_output.get(idx).unwrap().to_owned() as usize;
            let mut selected_item = state.map.get(item_id as usize).unwrap().to_owned();

            // create ata for receiver
            anchor_spl::associated_token::create(CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                anchor_spl::associated_token::Create {
                    payer: ctx.accounts.user.to_account_info(),
                    associated_token: remaining_accounts
                        .get((random as usize) * 3 + 2)
                        .unwrap()
                        .to_owned(),
                    authority: ctx.accounts.user.to_account_info(),
                    mint: remaining_accounts[(random as usize) * 3].to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
            ))?;

            // transfer nft
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: remaining_accounts[random * 3 + 1].to_account_info(),
                    to: remaining_accounts[random * 3 + 2].to_account_info(),
                    authority: ctx.accounts.auth.to_account_info(),
                },
                &seeds,
            );
            anchor_spl::token::transfer(cpi_ctx, 1)?;

            selected_item.owner = Some(ctx.accounts.user.key());
            selected_item.gacha = true;
            state.item_sold += 1;
        }

        // transfer fee to seller map[0]
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.user.key(),
                &ctx.accounts.seller.key(),
                _fee.try_into().unwrap(),
            ),
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.seller.to_account_info(),
            ],
        )?;

        Ok(())
    }

    pub fn create_gacha<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, CreateGacha<'info>>,
        _qty: u8,
        _bump: u8,
    ) -> Result<Vec<u128>> {
        let state = &mut ctx.accounts.state_account;
        let remaining_accounts = &mut ctx.remaining_accounts;

        let item_count = state.item_ids;
        let mut current_index = 0;

        let mut items = Vec::new();
        for i in 0..item_count {
            let item = state.map.get(i as usize).unwrap().to_owned();
            if item.owner == None {
                items.insert(current_index, item);
                current_index += 1;
            }
        }

        let mut gacha_items = Vec::new();
        let mut gacha_items_ids = Vec::new();
        require!(items.len() > 0, error::Error::ItemsUnavailableForGacha);
        require!(
            usize::from(_qty) <= items.len(),
            error::Error::InvalidQuantity
        );

        let mut random_output: Vec<u64> = Vec::new();
        let mut seed = Clock::get()?.unix_timestamp as u64;

        let range = (items.len()) as u64;
        let mut index = 0;
        for i in 0.._qty {
            seed ^= seed >> 12;
            seed ^= seed << 25;
            seed ^= seed >> 27;
            // seed *= 0x2545F4914F6CDD1D;
            loop {
                index = ((generate_random_f64(seed) * 1000.0) as u64) % range;
                if random_output.contains(&index) == false {
                    break;
                }
                seed += 16454654645667;
            }

            let item = items.get(*&index as usize).unwrap().to_owned();
            gacha_items.insert(i.into(), item.clone());
            gacha_items_ids.insert(i.into(), item.item_id);
            random_output.push(index);
        }

        let seeds = vec![_bump];
        let seeds = vec![b"auth".as_ref(), seeds.as_slice()];
        let seeds = vec![seeds.as_slice()];
        let seeds = seeds.as_slice();
        for idx in 0..(_qty as usize) {
            let random = random_output.get(idx).unwrap().to_owned();
            let item_id = gacha_items.get(idx).unwrap().to_owned().item_id;

            anchor_spl::associated_token::create(CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                anchor_spl::associated_token::Create {
                    payer: ctx.accounts.user.to_account_info(),
                    // associated_token: ctx.accounts.buyer_token_account.to_account_info(),
                    associated_token: remaining_accounts
                        .get((random as usize) * 3 + 2)
                        .unwrap()
                        .to_owned(),
                    authority: ctx.accounts.user.to_account_info(),
                    mint: remaining_accounts[(random as usize) * 3].to_account_info(),
                    // mint: ctx.accounts.mint.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
            ))?;

            // transfer nft
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    // from: ctx.accounts.from_token_account.to_account_info(),
                    from: remaining_accounts[(random as usize) * 3 + 1].to_account_info(),
                    // to: ctx.accounts.buyer_token_account.to_account_info(),
                    to: remaining_accounts[(random as usize) * 3 + 2].to_account_info(),
                    authority: ctx.accounts.auth.to_account_info(),
                },
                &seeds,
            );
            anchor_spl::token::transfer(cpi_ctx, 1)?;

            let mut item = state.map.get(item_id as usize).unwrap().to_owned();
            item.owner = Some(state.owner.key());
            item.gacha = true;

            state.item_sold += 1;

            // tranfer sol
            anchor_lang::solana_program::program::invoke(
                &anchor_lang::solana_program::system_instruction::transfer(
                    &ctx.accounts.user.key(),
                    &ctx.accounts.owner.key(),
                    state.listing_price.try_into().unwrap(),
                ),
                &[
                    ctx.accounts.user.to_account_info(),
                    ctx.accounts.owner.to_account_info(),
                ],
            )?;
        }

        Ok(gacha_items_ids)
    }
}

// convert the u64 into a double with range 0..1
fn generate_random_f64(seed: u64) -> f64 {
    let tmp = 0x3FF0000000000000 | (seed & 0xFFFFFFFFFFFFF);
    let result: f64 = unsafe { mem::transmute(tmp) };

    return result - 1.0;
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MarketItem {
    item_id: u128,
    // token_program_id: Pubkey, // program id,
    // mint_address: Pubkey,     // ATA
    // seller: Pubkey,
    owner: Option<Pubkey>,
    price: u128,
    // file_name: String,
    // description: String,
    // cash_back: u8,
    sold: bool,
    gacha: bool,
}

#[account]
pub struct State {
    pub map: Vec<MarketItem>,
    pub item_ids: u128,
    pub item_sold: u128,
    pub owner: Pubkey,
    pub listing_price: u128,
    pub initialized: bool,
}

#[derive(Accounts)]
pub struct InitState<'info> {
    /// CHECK:
    #[account(init, payer = user, space = 10240)]
    pub state_account: Account<'info, State>,

    #[account(mut)]
    pub user: Signer<'info>, // admin

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateMarketItem<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // seller

    #[account(mut)]
    pub state_account: Account<'info, State>,

    /// CHECK:
    #[account(mut)]
    pub to: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    pub from: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Purchase<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // seller

    pub token_program: Program<'info, Token>,

    #[account(mut)]
    pub state_account: Account<'info, State>,

    #[account(mut)]
    pub from_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub to_token_account: Account<'info, TokenAccount>,

    /// CHECK: token account authority PDA
    #[account(
        // seeds = ["auth".as_bytes().as_ref()],
        // bump,
    )]
    pub auth: UncheckedAccount<'info>,

    /// CHECK:
    #[account(mut)]
    pub seller: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Gacha<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // seller

    #[account(mut)]
    pub state_account: Account<'info, State>,

    /// CHECK:
    #[account(mut)]
    pub seller: AccountInfo<'info>,

    /// CHECK: token account authority PDA
    #[account()]
    pub auth: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct CreateGacha<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // seller

    #[account(mut)]
    pub state_account: Account<'info, State>,

    /// CHECK:
    #[account(mut)]
    pub seller: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    pub owner: AccountInfo<'info>,

    /// CHECK: token account authority PDA
    #[account()]
    pub auth: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
