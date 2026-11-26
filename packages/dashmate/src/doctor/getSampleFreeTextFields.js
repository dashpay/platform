/**
 * Samples whose fields doctor reads back as facts it decides on. In these,
 * only the free-text fields listed are scrubbed of secrets and the operator's
 * identity; the rest - a provider, an outcome, a time - is what the analysers
 * decide on, live and from an archived report, and a secret spelled like one
 * of them must not change it.
 */
const SAMPLE_FREE_TEXT_FIELDS = {
  gateway: {
    certificateRenewal: ['detail', 'error', 'path'],
  },
};

/**
 * @param {string} service
 * @param {string} key - the sample's name
 * @return {string[]|null} the fields that may be scrubbed, or null when every
 *   string in the sample may be
 */
export default function getSampleFreeTextFields(service, key) {
  return SAMPLE_FREE_TEXT_FIELDS[service]?.[key] ?? null;
}
