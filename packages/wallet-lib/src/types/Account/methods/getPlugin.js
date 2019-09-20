const {
  UnknownPlugin,
} = require('../../../errors');
/**
 * Get a plugin by name
 * @param pluginName
 * @return {*}
 */
function getPlugin(pluginName) {
  const loweredPluginName = pluginName.toLowerCase();
  const stdPluginsList = Object.keys(this.plugins.standard).map((key) => key.toLowerCase());
  if (stdPluginsList.includes(loweredPluginName)) {
    return this.plugins.standard[loweredPluginName];
  }
  throw new UnknownPlugin(loweredPluginName);
}
module.exports = getPlugin;
