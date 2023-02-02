use anchor_lang::error_code;

#[error_code]
pub enum Error {
    #[msg("Price must be at least 1 lamports")]
    InvalidPrice,
    #[msg("Invalid quantity")]
    InvalidQuantity,
    #[msg("Cash back should lower than 1")]
    CashbackMax,
    #[msg("Please submit the asking price in order to complete the purchase")]
    InvalidPayment,
    #[msg("Invalid account")]
    InvalidStateAccount,
    #[msg("State already has been initialized")]
    StateAlreadyInitialized,
    #[msg("Item list is not available for gacha")]
    ItemsUnavailableForGacha,
}