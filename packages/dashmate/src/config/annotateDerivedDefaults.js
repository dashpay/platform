import lodash from 'lodash';
import { inspect } from 'util';

import { DERIVED_DEFAULTS } from './derivedDefaults.js';

/**
 * An effective value the operator has not pinned, tagged so human-readable
 * output can say so.
 *
 * Rendering goes through the caller's own inspect, so the value is styled like
 * any other and the note sits outside the quotes where it cannot be mistaken for
 * part of the value.
 */
class TrackedDefault {
  constructor(value) {
    this.value = value;
  }

  [inspect.custom](depth, options, nodeInspect) {
    return `${nodeInspect(this.value, options)} ${options.stylize('(default)', 'undefined')}`;
  }
}

/**
 * Mark every option still using its version-derived default.
 *
 * For display only. Machine-readable output must not be annotated, and neither
 * must anything that gets persisted.
 *
 * @param {Config} config
 * @param {Object} options - effective options
 * @return {Object} a copy with unpinned values wrapped
 */
export default function annotateDerivedDefaults(config, options) {
  const annotated = lodash.cloneDeep(options);

  Object.keys(DERIVED_DEFAULTS)
    .filter((optionPath) => config.getStored(optionPath) === null)
    .forEach((optionPath) => {
      lodash.set(annotated, optionPath, new TrackedDefault(lodash.get(annotated, optionPath)));
    });

  return annotated;
}
