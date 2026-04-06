use anchor_lang::prelude::*;

declare_id!("Expnse1111111111111111111111111111111111111");

#[program]
pub mod expense_program {
    use super::*;

    pub fn init_user_profile(ctx: Context<InitUserProfile>) -> Result<()> {
        let profile = &mut ctx.accounts.user_profile;
        profile.owner = ctx.accounts.owner.key();
        profile.bump = ctx.bumps.user_profile;
        Ok(())
    }

    pub fn create_category(ctx: Context<CreateCategory>, name: String) -> Result<()> {
        require!(!name.trim().is_empty(), ExpenseError::InvalidCategoryName);
        require!(name.len() <= Category::MAX_NAME, ExpenseError::InvalidCategoryName);

        let category = &mut ctx.accounts.category;
        category.owner = ctx.accounts.owner.key();
        category.name = name;
        category.bump = ctx.bumps.category;
        Ok(())
    }

    pub fn create_expense(
        ctx: Context<CreateExpense>,
        expense_id: u64,
        amount: u64,
        note_hash: [u8; 32],
    ) -> Result<()> {
        require!(amount > 0, ExpenseError::InvalidAmount);
        require!(
            ctx.accounts.category.owner == ctx.accounts.owner.key(),
            ExpenseError::Unauthorized
        );

        let expense = &mut ctx.accounts.expense;
        expense.owner = ctx.accounts.owner.key();
        expense.expense_id = expense_id;
        expense.category = ctx.accounts.category.key();
        expense.amount = amount;
        expense.note_hash = note_hash;
        expense.status = ExpenseStatus::Pending;
        expense.bump = ctx.bumps.expense;
        Ok(())
    }

    pub fn update_expense_status(
        ctx: Context<UpdateExpenseStatus>,
        status: ExpenseStatus,
    ) -> Result<()> {
        let expense = &mut ctx.accounts.expense;
        require!(expense.owner == ctx.accounts.owner.key(), ExpenseError::Unauthorized);
        require!(
            expense.status == ExpenseStatus::Pending,
            ExpenseError::InvalidStatusTransition
        );
        expense.status = status;
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum ExpenseStatus {
    Pending,
    Approved,
    Rejected,
}

#[account]
pub struct UserProfile {
    pub owner: Pubkey,
    pub bump: u8,
}

impl UserProfile {
    pub const LEN: usize = 8 + 32 + 1;
}

#[account]
pub struct Category {
    pub owner: Pubkey,
    pub name: String,
    pub bump: u8,
}

impl Category {
    pub const MAX_NAME: usize = 64;
    pub const LEN: usize = 8 + 32 + 4 + Self::MAX_NAME + 1;
}

#[account]
pub struct Expense {
    pub owner: Pubkey,
    pub expense_id: u64,
    pub category: Pubkey,
    pub amount: u64,
    pub note_hash: [u8; 32],
    pub status: ExpenseStatus,
    pub bump: u8,
}

impl Expense {
    pub const LEN: usize = 8 + 32 + 8 + 32 + 8 + 32 + 1 + 1;
}

#[derive(Accounts)]
pub struct InitUserProfile<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = UserProfile::LEN,
        seeds = [b"user_profile", owner.key().as_ref()],
        bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateCategory<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [b"user_profile", owner.key().as_ref()],
        bump = user_profile.bump,
        constraint = user_profile.owner == owner.key() @ ExpenseError::Unauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(
        init,
        payer = owner,
        space = Category::LEN,
        seeds = [b"category", owner.key().as_ref(), name.as_bytes()],
        bump
    )]
    pub category: Account<'info, Category>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(expense_id: u64)]
pub struct CreateExpense<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [b"user_profile", owner.key().as_ref()],
        bump = user_profile.bump,
        constraint = user_profile.owner == owner.key() @ ExpenseError::Unauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(
        constraint = category.owner == owner.key() @ ExpenseError::Unauthorized
    )]
    pub category: Account<'info, Category>,
    #[account(
        init,
        payer = owner,
        space = Expense::LEN,
        seeds = [b"expense", owner.key().as_ref(), &expense_id.to_le_bytes()],
        bump
    )]
    pub expense: Account<'info, Expense>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateExpenseStatus<'info> {
    pub owner: Signer<'info>,
    #[account(mut)]
    pub expense: Account<'info, Expense>,
}

#[error_code]
pub enum ExpenseError {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Unauthorized operation")]
    Unauthorized,
    #[msg("Invalid category name")]
    InvalidCategoryName,
    #[msg("Invalid status transition")]
    InvalidStatusTransition,
}
