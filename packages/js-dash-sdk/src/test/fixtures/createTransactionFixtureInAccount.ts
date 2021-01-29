import { Transaction } from "@dashevo/dashcore-lib";

export async function createTransactionInAccount(account) {
    // add fake tx to the wallet so it will be able to create transactions
    const walletTransaction = new Transaction(undefined)
        .from([{
            amount: 150000,
            script: '76a914f9996443a7d5e2694560f8715e5e8fe602133c6088ac',
            outputIndex: 0,
            txid: new Transaction(undefined).hash,
        }])
        .to(account.getAddress(10).address, 100000);

    await account.importTransactions([walletTransaction.serialize(true)]);

    return walletTransaction;
}