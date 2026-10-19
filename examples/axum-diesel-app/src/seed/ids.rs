use uuid::Uuid;

/// Alice: holds the global wildcard `*`.
pub const ALICE: Uuid = Uuid::from_u128(0xaaaaaaaa_aaaa_aaaa_aaaa_aaaaaaaaaaaa);
/// Bob: holds the `Member` role within workspace A.
pub const BOB: Uuid = Uuid::from_u128(0xbbbbbbbb_bbbb_bbbb_bbbb_bbbbbbbbbbbb);
/// Carol: holds nothing.
pub const CAROL: Uuid = Uuid::from_u128(0xcccccccc_cccc_cccc_cccc_cccccccccccc);
/// Workspace A.
pub const WORKSPACE_A: Uuid = Uuid::from_u128(0x11111111_1111_1111_1111_111111111111);
/// Workspace B.
pub const WORKSPACE_B: Uuid = Uuid::from_u128(0x22222222_2222_2222_2222_222222222222);
/// An invoice in workspace A.
pub const INVOICE_A: Uuid = Uuid::from_u128(0xa1a1a1a1_a1a1_a1a1_a1a1_a1a1a1a1a1a1);
/// An invoice in workspace B.
pub const INVOICE_B: Uuid = Uuid::from_u128(0xb1b1b1b1_b1b1_b1b1_b1b1_b1b1b1b1b1b1);
/// Name of the role created by the seed.
pub const MEMBER_ROLE: &str = "Member";
