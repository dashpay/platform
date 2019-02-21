const Revisions = require('../../../../lib/stateView/revisions/Revisions');
const Revision = require('../../../../lib/stateView/revisions/Revision');

const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('Revisions', () => {
  let newRevisions;
  let oldRevisions;
  let previousRevisions;

  beforeEach(function beforeEach() {
    newRevisions = new Revisions(
      getReferenceFixture(2),
      [],
    );

    newRevisions.getRevisionNumber = this.sinon.stub();

    previousRevisions = [
      new Revision(0, getReferenceFixture(1)),
      new Revision(1, getReferenceFixture(2)),
      new Revision(2, getReferenceFixture(2)),
      new Revision(3, getReferenceFixture(2)),
    ];

    oldRevisions = new Revisions(
      getReferenceFixture(2),
      previousRevisions,
    );

    oldRevisions.getRevisionNumber = this.sinon.stub();
  });

  it('should be able to add and get revisions', () => {
    oldRevisions.getRevisionNumber.returns(4);

    const result = newRevisions.addRevision(oldRevisions);

    expect(result).to.equal(newRevisions);

    expect(newRevisions.getPreviousRevisions()).to.deep.equal(
      previousRevisions.concat([oldRevisions.getCurrentRevision()]),
    );
  });

  it('should remove revisions that are ahead of the current one', () => {
    newRevisions.getRevisionNumber.returns(2);

    newRevisions.addRevision(oldRevisions);

    newRevisions.removeAheadRevisions();

    expect(newRevisions.getPreviousRevisions()).to.deep.equal(
      previousRevisions.slice(0, 2),
    );
  });
});
