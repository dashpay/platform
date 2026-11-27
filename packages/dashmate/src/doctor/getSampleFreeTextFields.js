/**
 * Samples whose fields doctor reads back as facts it decides on. In these,
 * only the free-text fields listed, with everything in them, are scrubbed of
 * secrets and the operator's identity; the rest - a status, a code, a
 * provider, a time, a fingerprint - is what the analysers decide on, live and
 * from an archived report, and a secret spelled like one of them must not
 * change it.
 */
const SAMPLE_FREE_TEXT_FIELDS = {
  gateway: {
    certificateRenewal: ['detail', 'error', 'path'],
    // The status, reason codes, dates and fingerprint are kept; the messages
    // are written for a person and name the files they are about
    installedCertificate: ['message'],
    // Its state, failure codes, dates and fingerprints are compared with
    // the installed certificate's and decided on; nothing decides on the
    // names in it or its serial number
    servedCertificate: ['identityError', 'issuer', 'serialNumber', 'subject', 'subjectAltName'],
    // The error is the check's outcome; what comes with it is shown, not
    // decided on
    ssl: ['data'],
  },
};

/**
 * A state Core reports, as an identifier
 */
const STATE = /^[A-Z][A-Z_]{0,63}$/;

/**
 * A Tenderdash node ID: twenty bytes in hex
 */
const NODE_ID = /^[0-9A-Fa-f]{40}$/;

/**
 * Samples Core and Tenderdash answer with, whose strings are scrubbed except
 * the facts doctor decides on. Each fact is named by the keys that lead to it
 * in the sample, none for the sample itself, and kept only when it looks like
 * what it is. What else these answers hold is not dashmate's to list, so it
 * is all treated as free text.
 */
const SAMPLE_FACTS = {
  core: {
    masternodeStatus: [
      { path: ['state'], shape: STATE },
      { path: ['dmnState', 'platformNodeID'], shape: NODE_ID },
    ],
  },
  // Node IDs are public, since Tenderdash announces its own to every peer,
  // and the ID registered for the masternode is compared with each of these
  drive_tenderdash: {
    nodeIdFromKey: [{ path: [], shape: NODE_ID }],
    status: [{ path: ['value', 'result', 'node_info', 'id'], shape: NODE_ID }],
  },
};

/**
 * @param {string} service
 * @param {string} key - the sample's name
 * @return {string[]|null} the fields that may be scrubbed, with everything in
 *   them, or null when every string in the sample may be
 */
export default function getSampleFreeTextFields(service, key) {
  return SAMPLE_FREE_TEXT_FIELDS[service]?.[key] ?? null;
}

/**
 * @param {string} service
 * @param {string} key - the sample's name
 * @return {function(string[], string): boolean|null} whether the string at
 *   a path in a sample otherwise scrubbed whole is a fact to keep, or null
 *   when the sample has none
 */
export function getSampleFactFilter(service, key) {
  const facts = SAMPLE_FACTS[service]?.[key];

  if (!facts) {
    return null;
  }

  return (path, value) => facts.some((fact) => fact.path.length === path.length
    && fact.path.every((segment, index) => segment === path[index])
    && fact.shape.test(value));
}
