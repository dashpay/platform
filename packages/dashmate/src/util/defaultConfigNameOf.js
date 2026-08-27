/**
 * The config dashmate acts on when a command names none, if that is knowable.
 *
 * Tolerant on purpose. This is asked from tasks, analysers and reports, some of
 * which are constructed in places that have no configuration file at all - a
 * collected archive from another machine, or a test exercising one task. An
 * unknown default is not a failure: it means every command keeps its explicit
 * `--config`, which is the safe direction. Printing a flag that was not needed
 * costs an operator nothing; omitting one that was points them at another node.
 *
 * @param {ConfigFile|Object|null|undefined} configFile
 * @return {string|null}
 */
export default function defaultConfigNameOf(configFile) {
  return configFile?.getDefaultConfigName?.() ?? null;
}
