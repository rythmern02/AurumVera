use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use anchor_spl::metadata::{
    create_metadata_accounts_v3,
    CreateMetadataAccountsV3,
    Metadata,
};
use mpl_token_metadata::types::{DataV2, Creator};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod gold_token_program {
    use super::*;

    // Initialize the gold token mint with specific metadata
    pub fn initialize_gold_mint(
        ctx: Context<InitializeGoldMint>, 
        name: String, 
        symbol: String, 
        origin: String,
        purity: f32,
        ethical_certification: String
    ) -> Result<()> {
        // Create mint with 9 decimal places for fractional gram representation
        token::initialize_mint(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::InitializeMint {
                    mint: ctx.accounts.mint.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                }
            ),
            9, // 9 decimal places for fractional grams
            ctx.accounts.authority.key(),
            Some(ctx.accounts.authority.key())
        )?;

        // Create metadata for the gold token
        let cpi_accounts = CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            mint_authority: ctx.accounts.authority.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            update_authority: ctx.accounts.authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.metadata_program.to_account_info(), 
            cpi_accounts
        );

        create_metadata_accounts_v3(
            cpi_ctx,
            DataV2 {
                name,
                symbol,
                uri: format!("https://goldtrack.io/metadata/{}", ctx.accounts.mint.key()),
                seller_fee_basis_points: 0,
                creators: Some(vec![Creator {
                    address: ctx.accounts.authority.key(),
                    verified: true,
                    share: 100,
                }]),
                collection: None,
                uses: None,
            },
            true,  // is_mutable
            true,  // update_authority_is_signer
            Some(HashMap::from([
                ("origin".to_string(), origin),
                ("purity".to_string(), purity.to_string()),
                ("ethical_certification".to_string(), ethical_certification),
            ]))
        )?;

        Ok(())
    }

    // Mint gold tokens representing specific gram weight
    pub fn mint_gold_tokens(
        ctx: Context<MintGoldTokens>, 
        amount: u64, 
        holding_period: Option<u64>
    ) -> Result<()> {
        // Validate mint authority
        require!(
            ctx.accounts.mint_authority.key() == ctx.accounts.mint.mint_authority()?, 
            ErrorCode::InvalidMintAuthority
        );

        // Mint tokens to recipient
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                &[]
            ),
            amount
        )?;

        // Optional: Set holding period for dynamic NFT transformation
        if let Some(period) = holding_period {
            let token_info_pda = TokenInfoPDA::derive(
                ctx.accounts.mint.key(), 
                ctx.accounts.token_account.key()
            );
            
            token_info_pda.set_holding_start(Clock::get()?.unix_timestamp);
            token_info_pda.set_nft_transformation_period(period);
        }

        Ok(())
    }

    // Transfer gold tokens with provenance tracking
    pub fn transfer_gold_tokens(
        ctx: Context<TransferGoldTokens>, 
        amount: u64
    ) -> Result<()> {
        // Perform token transfer
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.from_token_account.to_account_info(),
                    to: ctx.accounts.to_token_account.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                }
            ),
            amount
        )?;

        // Update provenance tracking
        let provenance_pda = ProvenancePDA::derive(
            ctx.accounts.mint.key(),
            ctx.accounts.from_token_account.key(),
            ctx.accounts.to_token_account.key()
        );
        
        provenance_pda.record_transfer(
            ctx.accounts.from_token_account.key(),
            ctx.accounts.to_token_account.key(),
            amount,
            Clock::get()?.unix_timestamp
        );

        Ok(())
    }

    // Transform tokens to NFT after holding period
    pub fn transform_to_nft(ctx: Context<TransformToNFT>) -> Result<()> {
        let token_info_pda = TokenInfoPDA::derive(
            ctx.accounts.mint.key(), 
            ctx.accounts.token_account.key()
        );

        let current_time = Clock::get()?.unix_timestamp;
        require!(
            current_time - token_info_pda.holding_start >= token_info_pda.nft_transformation_period?,
            ErrorCode::InsufficientHoldingPeriod
        );

        // Logic to convert token to NFT
        // This would involve burning fungible tokens and minting an equivalent NFT
        // Detailed NFT minting logic would be implemented here

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeGoldMint<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = payer,
        mint::decimals = 9,
        mint::authority = authority.key()
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Metadata account
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintGoldTokens<'info> {
    #[account(mut)]
    pub mint_authority: Signer<'info>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = recipient
    )]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferGoldTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = owner
    )]
    pub from_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = recipient
    )]
    pub to_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransformToNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = owner
    )]
    pub token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Custom error codes for the program
#[derive(Code, FromPrimitive)]
pub enum ErrorCode {
    InvalidMintAuthority,
    InsufficientHoldingPeriod,
}

// Helper PDA for tracking token information
#[derive(Account)]
pub struct TokenInfoPDA {
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub holding_start: i64,
    pub nft_transformation_period: Option<u64>,
}

impl TokenInfoPDA {
    pub fn derive(mint: Pubkey, token_account: Pubkey) -> Self {
        let (pda, _bump) = Pubkey::find_program_address(
            &[b"token_info", mint.as_ref(), token_account.as_ref()],
            &ID
        );
        // Detailed PDA initialization would be implemented here
    }
}

// Provenance tracking PDA
#[derive(Account)]
pub struct ProvenancePDA {
    pub mint: Pubkey,
    pub transfers: Vec<TransferRecord>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TransferRecord {
    from: Pubkey,
    to: Pubkey,
    amount: u64,
    timestamp: i64,
}

impl ProvenancePDA {
    pub fn derive(mint: Pubkey, from: Pubkey, to: Pubkey) -> Self {
        let (pda, _bump) = Pubkey::find_program_address(
            &[b"provenance", mint.as_ref(), from.as_ref(), to.as_ref()],
            &ID
        );
        // Detailed PDA initialization would be implemented here
    }

    pub fn record_transfer(&mut self, from: Pubkey, to: Pubkey, amount: u64, timestamp: i64) {
        self.transfers.push(TransferRecord {
            from,
            to,
            amount,
            timestamp,
        });
    }
}