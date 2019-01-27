const Revision = require('./Revision');
const Reference = require('./Reference');

/**
 * Create revisions
 *
 * @param {{revision: number, reference: Object}[]} revisions
 * @returns {Revision[]}
 */
function createRevisions(revisions = []) {
  return revisions.map(({ revision, reference }) => (
    new Revision(
      revision,
      new Reference(reference),
    )
  ));
}

module.exports = createRevisions;
