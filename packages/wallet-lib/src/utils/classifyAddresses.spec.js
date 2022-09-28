const {expect} = require('chai');
const {WALLET_TYPES} = require('../CONSTANTS');
const classifyAddresses = require('./classifyAddresses');
const getFixtureHDAccountWithStorage = require("../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage");

const mockedHDAccount = getFixtureHDAccountWithStorage();

describe('Utils - classifyAddresses', function suite() {
  it('should correctly classify address for HDWallet', function () {
    const walletType = WALLET_TYPES.HDWALLET;
    const accountIndex = 0;
    const result = classifyAddresses(mockedHDAccount.storage.getWalletStore(mockedHDAccount.walletId), accountIndex, walletType);
    const expectedResult = {
      "externalAddressesList": ["yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7", "yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN", "ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE", "ybuL6rM6dgrKzCg8s99f3jxGuv5oz5JcDA", "ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2", "yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd", "yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv", "yhaAB6e8m3F8zmGX7WAVYa6eEfmSrrnY8x", "yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz", "yQYv3Um6DsdtANo1ZPTUte75wAGMstLRex", "yiYPJmu7eEm1cXUNumQRdjv1fvPhsfgMS4", "yii4aUZhNfL6EWN9KAgAFrJzGJmqHnF4wx", "yLpTquSct2SGz2Ka45uTPDd81Kzro2Jt2k", "yMiJtpzb1Qthy9TGnavsf5NZ6EZZa4j9q3", "yacgSfW7RkwWakEZPg8USAVdzCypiG3vxS", "yVvrmoRPFLy6nUpCQBT8ZExxF5wF3DhiGU", "yaJf2aG6cFUtfv4o6TuEKsh5kr4xq5iAY4", "yfardJQ4ucgWLKQPaRHGMRMbSGm5H4ExJR", "yLSCqx7dcM5JKR2fG7vHbF2axMvuYqomaw", "yVij8XpJ78LM5hepSV1KF7T8vRpUEXCpK5", "ydJpjuJGossAZR7S5oS7cWvjygEwoj8Xwp", "yW3TmWnmhvpxRbgFcQ8oXqDRkn3RhRH6jj", "yRegVX85DThKRkH8C61TtRacfzrkiBfNy5", "yPtDCqDFRe1JuDp8pvdiEMQMz2erGwS3VG", "yM9pSw3L4oBfG7uQL5o522Hu3WTvy9awgZ", "yNC6qYJYungzuk5XUynDFKCn54Dy8ngox4", "yR5KcLr1bceLT4teTk2qoJx6pFLik1zyzL", "yRrKLGJa9JmdjBWvrHtedKjHTao6CRDTKf", "yP5dShZBydpbEzgGoXL6kcjv2KzervRrYB"],
      "internalAddressesList": ["yNDpPsJqXKM36zHSNEW7c1zSvNnrZ699FY", "yLk4Hw3w4zDudrDVP6W8J9TggkY57zQUki", "yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL", "yhdRfg5gNr587dtEC4YYMcSHmLVEGqqtHc", "yYwKP1FQae5kbjXkmuirGx6Xzf8NzHpLqW", "yX9gmsm8aSxZZjYhq4w35aidT7qbhcpNjU", "ybgXCTGMHEBbQeUib8c3xAjtGAc12XtWiU", "yS31WpdMT2b34uL9C37fbUoACHhiupHCyP", "yTSpFqRoX3vyN286AUtKKhgmX5Xb41YKQe", "yQU5YsqN7psTTASuYbcMi7N5nNZGaxXb2X", "yVGGFj9BLgEab5rucSGLC6UGVLQKB4U1wJ", "yQCh5yYCHEbJzgSJE9rdHiqXHidKm3kwr5", "yX7T3Ac3yaLk5CTC5UaR93Fc7SjYkeT5hn", "yXx3WXq8kYNPbYEg5U6bL8Xfih4g5LCYVo", "yYnLMTz3jCi2KKKNuo3TVkEAGyUFg8tgkJ", "yiKa1dA6B4tSTNJqJP9Y5pQfQEffnQQDTL", "yf7vcuDnE9DVhXdMfBMQQTEi43otYQzkWE", "yTmSmocwERCeRHqNNG5SbpYKUra1HTmj8m", "yivUe5NeJsGsREwPQZUGYaTSwWB3E1oLcz", "ygfsZojdfW9UjCRU4ra95Aq6YgCC7UqZFx", "yU9fdXaUVtefwDZvxjJAr9xj1z2MtYi34A", "yXgMN6FgrgZCnTN1vhoZMh8afKMBmi3JC4", "yiqaCbXscvR8y3VFYMzdaKCaAGuDuZxMzt", "ydcgWDxheSxrLAqDBP4JXBndMCzUNf77gq"],
      "otherAccountAddressesList": ["yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2", "yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY", "yNPbYz5cZKw2EwxtkL3VSVzPi2FYp9VKjQ", "ybsGWzsnSCAZufgSeUjScVxqEdved99UM2", "yfNHuPojk8XKWP5nuueDptX4nM7qToudgx", "yXxLnDkk6s8h1PSnYaFM6MAyRarc1Kc1rY", "yipResSzN2zUvL7UYkmptKKmQTv7sNssRn", "yZPtNwimHdRiKYbNQW49qezw1Kc1YwUJeT", "yPMjYYfQbga2nBiuqqfUyX41U1vwRZ8fG8", "yLueLWWcLQsaXQ8D5o9tcyo8tfTxMWXvG4", "yN8gzgsc1RVjXThMQT5qZH2jjpnMymz6zP", "yPQLWBNwMdLxUW2oUwHGwQtfyYxD41BARJ", "yg5g2AfWFdwWexWGfbSXYbUHf1y5WWrFPs", "yWyABu4naV1Jzw7w9sn1gqhebPRSkCndsS", "ycuUPzUBjhKyUjezQR1LNot79a6C4aRLaR", "yQ7YjvAXgDAUCekveHVjr6NBveXrUemVno", "yi8bghcw627cMGpuH4bJqH6bqR5ywv1NLH", "yizHu8i2rfwzwBgnJ62s2WUe6wLoDjne6N", "yW1u3tySeUKAKJsz7sjZFyjUiTyKLB6xBv", "yNaSkdy1Q8JNubUdbLMGsGf7sTRofEJYZq", "yXMrw79LPgu78EJsfGGYpm6fXKc1EMnQ49", "yh6Hcyipdvp6WJpQxjNbaXP4kzPQUJpY3n", "yNphpXuaTZRpU9FBh2W7NkUYcr3kBDE8me", "yXFppDT59xYD41mT2pmAdnvr7aZEFdgdrN", "yeKGAiiEHBGRujvLoYewA77jDDpeDamxvF", "yaxTG66CVzKgHhHZXojRHC9ztLTvz3fwdT", "yYw6qU7dwGoELZkSTj3oSKRpM4U8qTMc1U", "yQE2MksEnSfbeNre19oja9Jj8tvpj64C5a", "yaRnvHo8oLvVmv46vMj5XPbDJouQSnmcLT", "yj5ofWf2uYQQkSavYm2WXgu1QkaZCyP3Cm", "yUCjGmEwrHJwNDrE1o2rMre6MkSbiE6yz7", "yfJzd1nE2rEqz5XEurD6vs4ykizwmw9xTv", "yUk8U3jRZMHKVTa1eFDEtZpa1G4E13FP4d", "yMr59YWQFCADq4FbWrtxDUtMwwshSrmAyK", "yetSehBupzGS9yps5ogqARUGmTMAs2xVcQ", "yNcESKLwriNrhM6EyoSpZEXrzdY3uht92T", "yN2FihGU7KdaEspp39bKrhsHypeyeYzoM2", "yirpWLxHuhwFzA6LfUPKUh1Ke9RB9BUjit", "yVDN66vvdshWdNzhUaQNB6xExAHkzs1zj8", "yPzofnEhRVfDisL2nCUJtAoSHkuyMirHZS"],
      "miscAddressesList": []
    }

    expect(result.externalAddressesList).to.deep.equal(expectedResult.externalAddressesList);
    expect(result.internalAddressesList).to.deep.equal(expectedResult.internalAddressesList);
    expect(result.otherAccountAddressesList).to.deep.equal(expectedResult.otherAccountAddressesList);
    expect(result.miscAddressesList).to.deep.equal(expectedResult.miscAddressesList);
  });
});
