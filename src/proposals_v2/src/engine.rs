//! Proposals engine module for the Joystream platform. Version 2.
//! Provides methods and extrinsics to create and vote for proposals.
//!
//! Supported extrinsics:
//! - vote
//!
//! Public API (requires root origin):
//! - create_proposal
//!


use rstd::collections::btree_set::BTreeSet;
use rstd::prelude::*;
use runtime_primitives::traits::EnsureOrigin;
use srml_support::{decl_module, decl_storage, dispatch};

use super::*;

//TODO: implement veto

/// Proposals engine trait.
pub trait Trait: system::Trait + timestamp::Trait {
    /// Origin from which proposals must come.
    type ProposalOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

    /// Origin from which votes must come.
    type VoteOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

    /// Provides data for voting. Defines maximum voters count for the proposal.
    type TotalVotersCounter: VotersParameters;

    /// Converts proposal code binary to executable representation
    type ProposalCodeDecoder: ProposalCodeDecoder;
}

// Storage for the proposals module
decl_storage! {
    trait Store for Module<T: Trait> as Proposals {
        /// Map proposal by its id.
        pub Proposals get(fn proposals): map u32 => Proposal<T::BlockNumber, T::AccountId>;

        /// Count of all proposals that have been created.
        pub ProposalCount get(fn proposal_count): u32;

        /// Map proposal executable code by proposal id.
        ProposalCode get(fn proposal_codes): map u32 =>  Vec<u8>;

        /// Map votes by proposal id.
        VotesByProposalId get(fn votes_by_proposal): map u32 => Vec<Vote<T::AccountId>>;

        /// Ids of proposals that are open for voting (have not been finalized yet).
        pub ActiveProposalIds get(fn active_proposal_ids): BTreeSet<u32>;

        /// Proposal tally results map
        pub(crate) TallyResults get(fn tally_results): map u32 => TallyResult<T::BlockNumber>;
    }
}

decl_module! {
    /// 'Proposal engine' substrate module
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        /// Vote extrinsic. Conditions:  origin must allow votes.
        pub fn vote(origin, proposal_id: u32, vote: VoteKind) -> dispatch::Result {
            let voter_id = T::VoteOrigin::ensure_origin(origin)?;

            Self::process_vote(voter_id, proposal_id, vote)
        }

        /// Block finalization. Perform voting period check and vote result tally.
        fn on_finalize(_n: T::BlockNumber) {
            let tally_results = Self::tally();

            // mutation

            for  tally_result in tally_results {
                <TallyResults<T>>::insert(tally_result.proposal_id, &tally_result);

                Self::update_proposal_status(tally_result.proposal_id, tally_result.status);
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// Create proposal. Requires root permissions.
    pub fn create_proposal(
        origin: T::Origin,
        //        proposer_id: T::AccountId,
        parameters: ProposalParameters<T::BlockNumber>,
        title: Vec<u8>,
        body: Vec<u8>,
        proposal_type: u32,
        proposal_code: Vec<u8>,
    ) -> dispatch::Result {
        let proposer_id = T::ProposalOrigin::ensure_origin(origin)?;

        let next_proposal_count_value = Self::proposal_count() + 1;
        let new_proposal_id = next_proposal_count_value;

        let new_proposal = Proposal {
            created: Self::current_block(),
            parameters,
            title,
            body,
            proposer_id,
            proposal_type,
            status: ProposalStatus::Active,
        };

        // mutation
        <Proposals<T>>::insert(new_proposal_id, new_proposal);
        <ProposalCode>::insert(new_proposal_id, proposal_code);
        ActiveProposalIds::mutate(|ids| ids.insert(new_proposal_id));
        ProposalCount::put(next_proposal_count_value);

        Ok(())
    }
}

impl<T: Trait> Module<T> {
    // Wrapper-function over system::block_number()
    fn current_block() -> T::BlockNumber {
        <system::Module<T>>::block_number()
    }

    // Actual vote processor. Stores votes for proposal.
    fn process_vote(voter_id: T::AccountId, proposal_id: u32, vote: VoteKind) -> dispatch::Result {
        let new_vote = Vote {
            voter_id,
            vote_kind: vote,
        };

        <VotesByProposalId<T>>::mutate(proposal_id, |votes| votes.push(new_vote));

        Ok(())
    }

    // Executes approved proposal code
    fn execute_proposal(proposal_id: u32) {
        //let origin = system::RawOrigin::Root.into();
        let proposal = Self::proposals(proposal_id);
        let proposal_code = Self::proposal_codes(proposal_id);

        let proposal_code_result =
            T::ProposalCodeDecoder::decode_proposal(proposal.proposal_type, proposal_code);

        let new_proposal_status = match proposal_code_result {
            Ok(proposal_code) => {
                if let Err(error) = proposal_code.execute() {
                    ProposalStatus::Failed {
                        error: error.as_bytes().to_vec(),
                    }
                } else {
                    ProposalStatus::Executed
                }
            }
            Err(error) => ProposalStatus::Failed {
                error: error.as_bytes().to_vec(),
            },
        };

        Self::update_proposal_status(proposal_id, new_proposal_status)
    }

    /// Voting results tally.
    /// Returns proposals with changed status and tally results
    fn tally() -> Vec<TallyResult<T::BlockNumber>> {
        let mut results = Vec::new();
        for &proposal_id in Self::active_proposal_ids().iter() {
            let votes = Self::votes_by_proposal(proposal_id);
            let proposal = Self::proposals(proposal_id);

            if let Some(tally_result) = proposal.tally_results(
                proposal_id,
                votes,
                T::TotalVotersCounter::total_voters_count(),
                Self::current_block(),
            ) {
                results.push(tally_result);
            }
        }

        results
    }

    /// Updates proposal status and removes proposal id from active id set.
    fn update_proposal_status(proposal_id: u32, new_status: ProposalStatus) {
        <Proposals<T>>::mutate(proposal_id, |p| p.status = new_status.clone());
        ActiveProposalIds::mutate(|ids| ids.remove(&proposal_id));

        match new_status {
            ProposalStatus::Rejected | ProposalStatus::Expired => {
                Self::reject_proposal(proposal_id)
            }
            ProposalStatus::Approved => Self::approve_proposal(proposal_id),
            ProposalStatus::Active => {
                // restore active proposal id
                ActiveProposalIds::mutate(|ids| ids.insert(proposal_id));
            }
            ProposalStatus::Executed | ProposalStatus::Failed { .. } => {} // do nothing
        }
    }

    /// Reject a proposal. The staked deposit will be returned to a proposer.
    fn reject_proposal(_proposal_id: u32) {}

    /// Approve a proposal. The staked deposit will be returned.
    fn approve_proposal(proposal_id: u32) {
        Self::execute_proposal(proposal_id);
    }
}
