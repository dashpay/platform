function testProofStructure(expect, proof) {
  expect(proof).to.exist();

  expect(proof.rootTreeProof).to.be.an.instanceof(Uint8Array);
  expect(proof.rootTreeProof.length).to.be.greaterThan(0);
  expect(proof.storeTreeProofs).to.exist();

  expect(proof.storeTreeProofs.getIdentitiesProof()).to.be.an.instanceof(Uint8Array);
  expect(proof.storeTreeProofs.getPublicKeyHashesToIdentityIdsProof())
    .to.be.an.instanceof(Uint8Array);
  expect(proof.storeTreeProofs.getDataContractsProof()).to.be.an.instanceof(Uint8Array);
  expect(proof.storeTreeProofs.getDocumentsProof()).to.be.an.instanceof(Uint8Array);

  expect(proof.signatureLLMQHash).to.be.an.instanceof(Uint8Array);
  expect(proof.signatureLLMQHash.length).to.be.equal(32);

  expect(proof.signature).to.be.an.instanceof(Uint8Array);
  expect(proof.signature.length).to.be.equal(96);
}

module.exports = testProofStructure;
