export default function extractCoreVersion(subversion) {
  return subversion.replace(/\/|\(.*?\)|Dash Core:/g, '').replace(/\/|\(.*?\)/g, '');
}
