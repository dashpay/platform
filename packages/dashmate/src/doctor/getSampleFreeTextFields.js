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
 * @param {string} service
 * @param {string} key - the sample's name
 * @return {string[]|null} the fields that may be scrubbed, with everything in
 *   them, or null when every string in the sample may be
 */
export default function getSampleFreeTextFields(service, key) {
  return SAMPLE_FREE_TEXT_FIELDS[service]?.[key] ?? null;
}
