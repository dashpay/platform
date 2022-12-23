function testProofStructure(expect, proof, proofExist = true) {
  expect(proof).to.exist();

  expect(proof.merkleProof).to.be.an.instanceof(Buffer);

  if (proofExist) {
    expect(proof.merkleProof.length).to.be.greaterThan(0);
  } else {
    expect(proof.merkleProof.length).to.be.equal(0);
  }

  expect(proof.quorumHash).to.be.an.instanceof(Buffer);
  expect(proof.quorumHash.length).to.be.equal(32);

  expect(proof.signature).to.be.an.instanceof(Buffer);
  expect(proof.signature.length).to.be.equal(96);

  expect(proof.round).to.be.a('number');
  expect(proof.round).to.be.greaterThanOrEqual(0);
}

module.exports = testProofStructure;
