//! FANatic Alpha — On-Chain Autonomous Agent Settlement
//!
//! Records trading signals immutably on-chain and settles agent positions
//! with full auditability. All operations use checked math for safety.

use anchor_lang::prelude::*;

declare_id!("CMso1JNiDvK23fTo9VK3RMGZah2iQLrAm1LKsR1pHdm1");

// ════════════════════════════════════════════════════════
// PDA SEEDS
// ════════════════════════════════════════════════════════

pub const AGENT_SEED: &[u8] = b"agent";
pub const SIGNAL_SEED: &[u8] = b"signal";
pub const POSITION_SEED: &[u8] = b"position";
pub const CONFIG_SEED: &[u8] = b"strategy";

// ════════════════════════════════════════════════════════
// INSTRUCTION 1: log_signal
// ════════════════════════════════════════════════════════

#[derive(Accounts)]
#[instruction(market_id: String, nonce: u64)]
pub struct LogSignal<'info> {
    #[account(mut)]
    pub agent: Signer<'info>,

    #[account(
        init,
        payer = agent,
        space = SignalLog::LEN,
        seeds = [SIGNAL_SEED, agent.key().as_ref(), market_id.as_bytes(), &nonce.to_le_bytes()],
        bump,
    )]
    pub signal_log: Account<'info, SignalLog>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct SignalLog {
    pub agent: Pubkey,
    pub market_id: String,
    pub strategy: u8,        // 0=SharpMove, 1=Arbitrage, 2=Steam
    pub confidence: u8,      // 0-100 representing 0.0-1.0
    pub odds_before: u64,    // Basis points
    pub odds_after: u64,     // Basis points
    pub timestamp: i64,
    pub nonce: u64,
    pub bump: u8,
    pub _reserved: [u8; 94],
}

impl SignalLog {
    pub const LEN: usize = 8 + 32 + (4 + 64) + 1 + 1 + 8 + 8 + 8 + 8 + 1 + 94;
}

pub fn log_signal(
    ctx: Context<LogSignal>,
    market_id: String,
    strategy: u8,
    confidence: u8,
    odds_before: u64,
    odds_after: u64,
    nonce: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let signal = &mut ctx.accounts.signal_log;
    signal.agent = ctx.accounts.agent.key();
    signal.market_id = market_id;
    signal.strategy = strategy;
    signal.confidence = confidence;
    signal.odds_before = odds_before;
    signal.odds_after = odds_after;
    signal.timestamp = clock.unix_timestamp;
    signal.nonce = nonce;
    signal.bump = ctx.bumps.signal_log;
    
    msg!("Signal logged: strategy={} confidence={}%", strategy, confidence);
    Ok(())
}

// ════════════════════════════════════════════════════════
// INSTRUCTION 2: open_position
// ════════════════════════════════════════════════════════

#[derive(Accounts)]
#[instruction(market_id: String)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub agent: Signer<'info>,

    #[account(
        init,
        payer = agent,
        space = AgentPosition::LEN,
        seeds = [POSITION_SEED, agent.key().as_ref(), market_id.as_bytes()],
        bump,
    )]
    pub position: Account<'info, AgentPosition>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct AgentPosition {
    pub agent: Pubkey,
    pub market_id: String,
    pub direction: u8,       // 0=Back, 1=Lay
    pub entry_odds: u64,     // Basis points
    pub stake_lamports: u64,
    pub status: u8,          // 0=Open, 1=Closed, 2=Settled
    pub opened_at: i64,
    pub closed_at: i64,
    pub pnl_lamports: i64,   // Negative = loss
    pub bump: u8,
    pub _reserved: [u8; 70],
}

impl AgentPosition {
    pub const LEN: usize = 8 + 32 + (4 + 64) + 1 + 8 + 8 + 1 + 8 + 8 + 8 + 1 + 70;
}

pub fn open_position(
    ctx: Context<OpenPosition>,
    market_id: String,
    direction: u8,
    entry_odds: u64,
) -> Result<()> {
    require!(direction <= 1, AlphaError::InvalidDirection);
    require!(entry_odds > 0, AlphaError::InvalidOdds);
    
    let clock = Clock::get()?;
    let pos = &mut ctx.accounts.position;
    pos.agent = ctx.accounts.agent.key();
    pos.market_id = market_id;
    pos.direction = direction;
    pos.entry_odds = entry_odds;
    pos.stake_lamports = 0;
    pos.status = 0;
    pos.opened_at = clock.unix_timestamp;
    pos.closed_at = 0;
    pos.pnl_lamports = 0;
    pos.bump = ctx.bumps.position;
    
    msg!("Position opened: market={} direction={}", pos.market_id, direction);
    Ok(())
}

// ════════════════════════════════════════════════════════
// INSTRUCTION 3: close_position
// ════════════════════════════════════════════════════════

#[derive(Accounts)]
#[instruction(market_id: String)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub agent: Signer<'info>,

    #[account(
        mut,
        seeds = [POSITION_SEED, agent.key().as_ref(), market_id.as_bytes()],
        bump = position.bump,
    )]
    pub position: Account<'info, AgentPosition>,
}

pub fn close_position(
    ctx: Context<ClosePosition>,
    _market_id: String,
    final_odds: u64,
) -> Result<()> {
    let pos = &mut ctx.accounts.position;
    require!(pos.status == 0, AlphaError::PositionAlreadyClosed);
    require!(final_odds > 0, AlphaError::InvalidOdds);
    
    let clock = Clock::get()?;
    
    // Calculate P&L in basis points
    let pnl_bps = if pos.direction == 0 {
        // Back: profit if odds shorten (entry > final)
        (pos.entry_odds as i64)
            .checked_sub(final_odds as i64)
            .ok_or(AlphaError::Overflow)?
    } else {
        // Lay: profit if odds lengthen (final > entry)
        (final_odds as i64)
            .checked_sub(pos.entry_odds as i64)
            .ok_or(AlphaError::Overflow)?
    };
    
    pos.pnl_lamports = pnl_bps;
    pos.closed_at = clock.unix_timestamp;
    pos.status = 1;
    
    msg!("Position closed: pnl={} bps", pnl_bps);
    Ok(())
}

// ════════════════════════════════════════════════════════
// ERRORS
// ════════════════════════════════════════════════════════

#[error_code]
pub enum AlphaError {
    #[msg("Invalid direction: must be 0 (Back) or 1 (Lay)")]
    InvalidDirection,
    #[msg("Invalid odds: must be greater than zero")]
    InvalidOdds,
    #[msg("Position already closed or settled")]
    PositionAlreadyClosed,
    #[msg("Arithmetic overflow")]
    Overflow,
}

// ════════════════════════════════════════════════════════
// PROGRAM ENTRY POINT
// ════════════════════════════════════════════════════════

#[program]
pub mod fanatic_alpha {
    use super::*;

    pub fn log_signal_handler(
        ctx: Context<LogSignal>,
        market_id: String,
        strategy: u8,
        confidence: u8,
        odds_before: u64,
        odds_after: u64,
        nonce: u64,
    ) -> Result<()> {
        log_signal(ctx, market_id, strategy, confidence, odds_before, odds_after, nonce)
    }

    pub fn open_position_handler(
        ctx: Context<OpenPosition>,
        market_id: String,
        direction: u8,
        entry_odds: u64,
    ) -> Result<()> {
        open_position(ctx, market_id, direction, entry_odds)
    }

    pub fn close_position_handler(
        ctx: Context<ClosePosition>,
        market_id: String,
        final_odds: u64,
    ) -> Result<()> {
        close_position(ctx, market_id, final_odds)
    }
}
