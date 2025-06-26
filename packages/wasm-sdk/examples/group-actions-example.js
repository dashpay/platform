// Example of using group action state transitions in the WASM SDK

import init, {
  // Group action functions
  createGroupStateTransitionInfo,
  createTokenEventBytes,
  createGroupAction,
  addGroupInfoToStateTransition,
  getGroupInfoFromStateTransition,
  createGroupMember,
  validateGroupConfig,
  calculateGroupActionApproval,
  createGroupConfiguration,
  
  // Group management functions from group_actions module
  createGroup,
  addGroupMember,
  removeGroupMember,
  createGroupProposal,
  voteOnProposal,
  executeProposal,
  fetchGroup,
  fetchGroupMembers,
  fetchGroupProposals,
  
  // State transition functions
  getStateTransitionType,
  calculateStateTransitionId,
  
  // SDK
  WasmSdk,
} from '../pkg/wasm_sdk.js';

// Initialize WASM
await init();

// Example 1: Create a group with initial members
async function createGroupExample() {
  console.log('=== Create Group Example ===');
  
  const creatorId = 'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF';
  const groupName = 'Development DAO';
  const description = 'DAO for managing development funds';
  const groupType = 'dao';
  const threshold = 3; // Require 3 approvals
  
  const initialMembers = [
    'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF',
    'H9sjVAaLhC3S5cKryFJx1qEchNoMnBvimgLbJBWgHmPR',
    'GpRyJPj6DMhZJJx8kWxYEoqhJx2NrvyPQaPDZnKxHtFG',
    'BhPStrn3tKKYgckYNaFW1w6XfeCYVHmeRaXhTJunPjQu',
    'DG8MwpbxG7dDW8Y1ZmfhxS9fweBFDH7WwWHwVq5tCigU'
  ];
  
  const identityNonce = 1;
  const signaturePublicKeyId = 0;
  
  // Create the group
  const stBytes = createGroup(
    creatorId,
    groupName,
    description,
    groupType,
    threshold,
    initialMembers,
    identityNonce,
    signaturePublicKeyId
  );
  
  console.log('Group creation state transition size:', stBytes.length, 'bytes');
  
  // Get transition info
  const stId = calculateStateTransitionId(new Uint8Array(stBytes));
  console.log('State transition ID:', stId);
  
  return stBytes;
}

// Example 2: Create a group with power-based voting
async function createPowerBasedGroupExample() {
  console.log('\n=== Power-Based Group Example ===');
  
  // Create members with different voting powers
  const members = [
    createGroupMember('FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF', 100), // 100 power
    createGroupMember('H9sjVAaLhC3S5cKryFJx1qEchNoMnBvimgLbJBWgHmPR', 75),  // 75 power
    createGroupMember('GpRyJPj6DMhZJJx8kWxYEoqhJx2NrvyPQaPDZnKxHtFG', 50),  // 50 power
    createGroupMember('BhPStrn3tKKYgckYNaFW1w6XfeCYVHmeRaXhTJunPjQu', 25),  // 25 power
  ];
  
  const requiredPower = 150; // Need 150 power to approve actions
  const memberPowerLimit = 100; // No single member can have more than 100 power
  
  // Validate the configuration
  const validation = validateGroupConfig(members, requiredPower, memberPowerLimit);
  console.log('Group validation:', validation);
  
  // Create group configuration
  const groupConfig = createGroupConfiguration(
    0, // position
    requiredPower,
    memberPowerLimit,
    members
  );
  
  console.log('Group configuration:', groupConfig);
  
  return groupConfig;
}

// Example 3: Create and vote on a proposal
async function groupProposalExample() {
  console.log('\n=== Group Proposal Example ===');
  
  const groupId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S3Qdq';
  const proposerId = 'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF';
  
  // Create a proposal for token transfer
  const title = 'Fund Development Team';
  const description = 'Transfer 1000 tokens to development team wallet for Q1 2024';
  const actionType = 'token_transfer';
  
  // Create token event data
  const eventBytes = createTokenEventBytes(
    'transfer',
    0, // token position
    1000.0, // amount
    'H9sjVAaLhC3S5cKryFJx1qEchNoMnBvimgLbJBWgHmPR', // recipient
    'Q1 2024 development funding' // note
  );
  
  const durationHours = 72; // 3 days to vote
  const identityNonce = 2;
  const signaturePublicKeyId = 0;
  
  // Create the proposal
  const proposalBytes = createGroupProposal(
    groupId,
    proposerId,
    title,
    description,
    actionType,
    eventBytes,
    durationHours,
    identityNonce,
    signaturePublicKeyId
  );
  
  console.log('Proposal created, size:', proposalBytes.length, 'bytes');
  
  // Now vote on the proposal
  const proposalId = 'proposal123'; // This would come from the created proposal
  const voterId = 'H9sjVAaLhC3S5cKryFJx1qEchNoMnBvimgLbJBWgHmPR';
  
  const voteBytes = voteOnProposal(
    proposalId,
    voterId,
    true, // approve
    'Looks good, let\'s fund the team!', // comment
    1, // voter's nonce
    0  // voter's signature key
  );
  
  console.log('Vote cast, size:', voteBytes.length, 'bytes');
  
  return { proposalBytes, voteBytes };
}

// Example 4: Group action with state transition info
async function groupActionWithStateTransition() {
  console.log('\n=== Group Action with State Transition ===');
  
  // Create group state transition info as proposer
  const groupInfo = createGroupStateTransitionInfo(
    1, // group contract position
    null, // no action ID yet (we're the proposer)
    true  // is proposer
  );
  
  console.log('Group info (proposer):', groupInfo);
  
  // Create a group action
  const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S3Qdq';
  const proposerId = 'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF';
  
  const eventBytes = createTokenEventBytes(
    'mint',
    0, // token position
    5000.0, // amount
    null, // no recipient for mint
    'Initial token mint for DAO treasury'
  );
  
  const actionBytes = createGroupAction(
    contractId,
    proposerId,
    0, // token position
    eventBytes
  );
  
  console.log('Group action created, size:', actionBytes.length, 'bytes');
  
  // Create info for someone voting on this action
  const actionId = 'action456'; // This would be the actual action ID
  const voterGroupInfo = createGroupStateTransitionInfo(
    1, // same group position
    actionId,
    false // not proposer, just voting
  );
  
  console.log('Group info (voter):', voterGroupInfo);
  
  return { groupInfo, actionBytes, voterGroupInfo };
}

// Example 5: Calculate approval status
async function calculateApprovalExample() {
  console.log('\n=== Calculate Approval Status ===');
  
  // Simulate approvals from different members
  const approvals = [
    { identityId: 'member1', power: 100, timestamp: Date.now() },
    { identityId: 'member2', power: 75, timestamp: Date.now() + 1000 },
    { identityId: 'member3', power: 50, timestamp: Date.now() + 2000 },
  ];
  
  const requiredPower = 200;
  
  // Calculate if approved
  const approvalStatus = calculateGroupActionApproval(approvals, requiredPower);
  console.log('Approval status:', approvalStatus);
  
  // Add another approval
  approvals.push({ identityId: 'member4', power: 30, timestamp: Date.now() + 3000 });
  
  // Recalculate
  const newStatus = calculateGroupActionApproval(approvals, requiredPower);
  console.log('Updated approval status:', newStatus);
  
  return newStatus;
}

// Example 6: Complex multi-sig scenario
async function complexMultiSigExample() {
  console.log('\n=== Complex Multi-Sig Scenario ===');
  
  // Create a multi-sig group for treasury management
  const groupId = 'treasury-multisig';
  const creatorId = 'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF';
  
  // Create group with weighted voting
  const stBytes = createGroup(
    creatorId,
    'Treasury Multi-Sig',
    'Multi-signature wallet for protocol treasury',
    'multisig',
    3, // Need 3 signatures
    [
      creatorId,
      'H9sjVAaLhC3S5cKryFJx1qEchNoMnBvimgLbJBWgHmPR',
      'GpRyJPj6DMhZJJx8kWxYEoqhJx2NrvyPQaPDZnKxHtFG',
      'BhPStrn3tKKYgckYNaFW1w6XfeCYVHmeRaXhTJunPjQu',
    ],
    1,
    0
  );
  
  console.log('Multi-sig group created');
  
  // Create a high-value transfer proposal
  const proposalBytes = createGroupProposal(
    groupId,
    creatorId,
    'Emergency Protocol Upgrade Funding',
    'Transfer 50,000 tokens to fund critical protocol security upgrade',
    'token_transfer',
    createTokenEventBytes(
      'transfer',
      0,
      50000.0,
      'SecurityTeamWallet123',
      'Critical security patch funding - approved by security audit'
    ),
    24, // 24 hours for emergency vote
    2,
    0
  );
  
  console.log('High-value proposal created');
  
  // Simulate multiple votes
  const votes = [];
  const voters = [
    { id: 'H9sjVAaLhC3S5cKryFJx1qEchNoMnBvimgLbJBWgHmPR', approve: true, comment: 'Critical for security' },
    { id: 'GpRyJPj6DMhZJJx8kWxYEoqhJx2NrvyPQaPDZnKxHtFG', approve: true, comment: 'Verified audit report' },
    { id: 'BhPStrn3tKKYgckYNaFW1w6XfeCYVHmeRaXhTJunPjQu', approve: false, comment: 'Need more details' },
  ];
  
  for (const voter of voters) {
    const voteBytes = voteOnProposal(
      'proposal789',
      voter.id,
      voter.approve,
      voter.comment,
      1,
      0
    );
    votes.push({ voter: voter.id, approve: voter.approve, size: voteBytes.length });
  }
  
  console.log('Votes collected:', votes);
  
  // Check if we have enough approvals (3 required, 2 approved)
  const approvedCount = votes.filter(v => v.approve).length;
  console.log(`Approval status: ${approvedCount}/3 signatures`);
  
  return { stBytes, proposalBytes, votes };
}

// Example 7: SDK integration
async function sdkIntegrationExample() {
  console.log('\n=== SDK Integration Example ===');
  
  const sdk = new WasmSdk();
  
  try {
    // Fetch group information
    const group = await fetchGroup(sdk, 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S3Qdq');
    console.log('Fetched group:', {
      id: group.id,
      name: group.name,
      type: group.groupType,
      memberCount: group.memberCount,
      threshold: group.threshold,
      active: group.active
    });
    
    // Fetch group members
    const members = await fetchGroupMembers(sdk, group.id);
    console.log('Group members:', members.length);
    
    // Fetch active proposals
    const proposals = await fetchGroupProposals(sdk, group.id, true);
    console.log('Active proposals:', proposals.length);
    
  } catch (error) {
    console.log('SDK operations would work with actual Platform connection');
  }
}

// Example 8: Group member management
async function memberManagementExample() {
  console.log('\n=== Member Management Example ===');
  
  const groupId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S3Qdq';
  const adminId = 'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF';
  
  // Add a new member
  const newMemberId = 'NewMember123456789';
  const addMemberBytes = addGroupMember(
    groupId,
    adminId,
    newMemberId,
    'member',
    ['vote', 'propose'], // permissions
    3, // nonce
    0  // signature key
  );
  
  console.log('Add member transaction size:', addMemberBytes.length, 'bytes');
  
  // Remove a member
  const removeMemberId = 'InactiveMember987654321';
  const removeMemberBytes = removeGroupMember(
    groupId,
    adminId,
    removeMemberId,
    4, // nonce
    0  // signature key
  );
  
  console.log('Remove member transaction size:', removeMemberBytes.length, 'bytes');
  
  return { addMemberBytes, removeMemberBytes };
}

// Run all examples
(async () => {
  try {
    await createGroupExample();
    await createPowerBasedGroupExample();
    await groupProposalExample();
    await groupActionWithStateTransition();
    await calculateApprovalExample();
    await complexMultiSigExample();
    await sdkIntegrationExample();
    await memberManagementExample();
    
    console.log('\n✅ All group action examples completed successfully!');
  } catch (error) {
    console.error('❌ Error in group action examples:', error);
  }
})();