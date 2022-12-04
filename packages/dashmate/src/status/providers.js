const fetch = require('node-fetch')
const MAX_REQUEST_TIMEOUT = 5000;

const request = async (url) => {
  try {
    return await fetch(url, {
      signal: AbortSignal.timeout(MAX_REQUEST_TIMEOUT),
    });
  } catch (e) {
    if (e.name === 'FetchError' || e.name === 'AbortError') {
      return null;
    } else {
      throw e;
    }
  }
}

const requestJSON = async (url) => {
  const response = await request(url)

  if (response) {
    return response.json()
  }

  return response
}

const requestText = async (url) => {
  const response = await request(url)

  return response.text()
}

const insightURLs = {
  testnet: 'https://testnet-insight.dashevo.org/insight-api',
  mainnet: 'https://insight.dash.org/insight-api',
};

module.exports = {
  insight: (chain) => ({
    status: async () => {
      if (!insightURLs[chain]) {
        return null
      }

      return requestJSON(`${insightURLs[chain]}/status`)
    }
  }),
  github: {
    release: async (repoSlug) => {
      const json = await requestJSON(`https://api.github.com/repos/${repoSlug}/releases/latest`)

      return json.tag_name.substring(1)
    },
  },
  mnowatch: {
    checkPortStatus: async (port) => requestText(`https://mnowatch.org/${port}/`)
  }
}
