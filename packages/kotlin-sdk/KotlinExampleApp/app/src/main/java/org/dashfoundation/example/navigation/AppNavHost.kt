package org.dashfoundation.example.navigation

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.toRoute
import org.dashfoundation.example.ui.contracts.ContractsHomeScreen
import org.dashfoundation.example.ui.contracts.CountDocumentsScreen
import org.dashfoundation.example.ui.contracts.CreateDocumentScreen
import org.dashfoundation.example.ui.contracts.DataContractDetailsScreen
import org.dashfoundation.example.ui.contracts.DocumentActionsScreen
import org.dashfoundation.example.ui.contracts.DocumentFieldsScreen
import org.dashfoundation.example.ui.contracts.DocumentTypeDetailsScreen
import org.dashfoundation.example.ui.contracts.DocumentWithPriceScreen
import org.dashfoundation.example.ui.contracts.DocumentsScreen
import org.dashfoundation.example.ui.contracts.GroveDBPathElementsScreen
import org.dashfoundation.example.ui.contracts.GroupDetailScreen
import org.dashfoundation.example.ui.contracts.LocalDataContractsScreen
import org.dashfoundation.example.ui.contracts.RegisterContractSourceScreen
import org.dashfoundation.example.ui.contracts.QueriesListScreen
import org.dashfoundation.example.ui.contracts.QueryDetailScreen
import org.dashfoundation.example.ui.contracts.SumAverageDocumentsScreen
import org.dashfoundation.example.ui.contracts.UpdateContractScreen
import org.dashfoundation.example.ui.diagnostics.AddressQueriesScreen
import org.dashfoundation.example.ui.diagnostics.BannedAddressesScreen
import org.dashfoundation.example.ui.diagnostics.DiagnosticsScreen
import org.dashfoundation.example.ui.diagnostics.KeystoreExplorerScreen
import org.dashfoundation.example.ui.diagnostics.WalletMemoryExplorerScreen
import org.dashfoundation.example.ui.identity.AddIdentityKeyScreen
import org.dashfoundation.example.ui.identity.ContestDetailScreen
import org.dashfoundation.example.ui.identity.CreateIdentityScreen
import org.dashfoundation.example.ui.dashpay.AddContactScreen
import org.dashfoundation.example.ui.dashpay.ContactDetailScreen
import org.dashfoundation.example.ui.dashpay.ContactRequestsScreen
import org.dashfoundation.example.ui.dashpay.ContactsScreen
import org.dashfoundation.example.ui.dashpay.DashPayProfileScreen
import org.dashfoundation.example.ui.dashpay.DashPayTabScreen
import org.dashfoundation.example.ui.dashpay.HiddenContactsScreen
import org.dashfoundation.example.ui.dashpay.IgnoredContactsScreen
import org.dashfoundation.example.ui.identity.DpnsTestScreen
import org.dashfoundation.example.ui.identity.IdentitiesHomeScreen
import org.dashfoundation.example.ui.identity.IdentityDetailScreen
import org.dashfoundation.example.ui.identity.KeyDetailScreen
import org.dashfoundation.example.ui.identity.KeysListScreen
import org.dashfoundation.example.ui.identity.LoadIdentityScreen
import org.dashfoundation.example.ui.identity.RegisterNameScreen
import org.dashfoundation.example.ui.identity.RegistrationProgressScreen
import org.dashfoundation.example.ui.identity.SearchWalletsForIdentitiesScreen
import org.dashfoundation.example.ui.identity.SelectMainNameScreen
import org.dashfoundation.example.ui.credits.TopUpIdentityFromCoreScreen
import org.dashfoundation.example.ui.credits.TopUpIdentityScreen
import org.dashfoundation.example.ui.credits.TransferCreditsScreen
import org.dashfoundation.example.ui.credits.TransferIdentityToAddressScreen
import org.dashfoundation.example.ui.credits.TransferPlatformAddressScreen
import org.dashfoundation.example.ui.credits.WithdrawCreditsScreen
import org.dashfoundation.example.ui.credits.WithdrawPlatformAddressScreen
import org.dashfoundation.example.ui.transitions.StateTransitionsScreen
import org.dashfoundation.example.ui.transitions.TransitionCategoryScreen
import org.dashfoundation.example.ui.transitions.TransitionDetailScreen
import org.dashfoundation.example.ui.funding.AddressFundProgressScreen
import org.dashfoundation.example.ui.funding.FundFromAssetLockScreen
import org.dashfoundation.example.ui.shielded.SeedShieldedPoolScreen
import org.dashfoundation.example.ui.shielded.ShieldedActivityScreen
import org.dashfoundation.example.ui.shielded.ShieldedFundProgressScreen
import org.dashfoundation.example.ui.shielded.ShieldedFundScreen
import org.dashfoundation.example.ui.settings.DataManagementScreen
import org.dashfoundation.example.ui.settings.SettingsScreen
import org.dashfoundation.example.ui.tokens.CoSignProposalScreen
import org.dashfoundation.example.ui.tokens.PendingGroupActionsScreen
import org.dashfoundation.example.ui.tokens.QuickBasicTokenScreen
import org.dashfoundation.example.ui.tokens.TokenActionPermissionsScreen
import org.dashfoundation.example.ui.tokens.TokenActionScreen
import org.dashfoundation.example.ui.tokens.TokenDetailsScreen
import org.dashfoundation.example.ui.tokens.TokenSearchScreen
import org.dashfoundation.example.ui.tokens.TokensScreen
import org.dashfoundation.example.ui.storage.StorageExplorerScreen
import org.dashfoundation.example.ui.storage.StorageModelListScreen
import org.dashfoundation.example.ui.storage.StorageRecordDetailScreen
import org.dashfoundation.example.ui.scanner.QrScannerScreen
import org.dashfoundation.example.ui.sync.SyncStatusScreen
import org.dashfoundation.example.ui.wallet.AccountDetailScreen
import org.dashfoundation.example.ui.wallet.CreateWalletScreen
import org.dashfoundation.example.ui.wallet.SeedBackupScreen
import org.dashfoundation.example.ui.wallet.SendTransactionScreen
import org.dashfoundation.example.ui.wallet.TransactionDetailScreen
import org.dashfoundation.example.ui.wallet.TransactionListScreen
import org.dashfoundation.example.ui.wallet.WalletDetailScreen
import org.dashfoundation.example.ui.wallet.WalletsScreen

/**
 * The app's navigation graph. One NavHost, destinations grouped by tab;
 * per-tab back stacks come from the save/restoreState navigation options in
 * MainScreen. Detail destinations register here as their feature milestones
 * land (routes in [Routes.kt]).
 */
@Composable
fun AppNavHost(
    navController: NavHostController,
    modifier: Modifier = Modifier,
) {
    NavHost(
        navController = navController,
        startDestination = SyncHome,
        modifier = modifier,
    ) {
        composable<SyncHome> { SyncStatusScreen() }

        composable<WalletsHome> { WalletsScreen(navController) }

        composable<CreateWallet> { CreateWalletScreen(navController) }

        composable<WalletDetail> { entry ->
            val route = entry.toRoute<WalletDetail>()
            WalletDetailScreen(route.walletIdHex, navController)
        }

        composable<SeedBackup> { entry ->
            val route = entry.toRoute<SeedBackup>()
            SeedBackupScreen(route.walletIdHex, navController)
        }

        composable<AccountDetail> { entry ->
            val route = entry.toRoute<AccountDetail>()
            AccountDetailScreen(route.walletIdHex, route.accountId, navController)
        }

        composable<SendTransaction> { entry ->
            val route = entry.toRoute<SendTransaction>()
            SendTransactionScreen(route.walletIdHex, navController)
        }

        composable<WalletTransactions> { entry ->
            val route = entry.toRoute<WalletTransactions>()
            TransactionListScreen(route.walletIdHex, navController)
        }

        composable<WalletTransactionDetail> { entry ->
            val route = entry.toRoute<WalletTransactionDetail>()
            TransactionDetailScreen(
                route.txidHex,
                navController,
                assetLockAmountDuffs = route.assetLockAmountDuffs
                    .takeIf { it != WalletTransactionDetail.AMOUNT_ABSENT },
            )
        }

        composable<QrScanner> { QrScannerScreen(navController) }

        // ── Identities graph ───────────────────────────────────────────

        composable<IdentitiesHome> { IdentitiesHomeScreen(navController) }

        composable<CreateIdentity> { CreateIdentityScreen(navController) }

        composable<LoadIdentity> { LoadIdentityScreen(navController) }

        composable<SearchWalletsForIdentities> { SearchWalletsForIdentitiesScreen(navController) }

        composable<DpnsTest> { DpnsTestScreen(navController) }

        composable<IdentityDetail> { entry ->
            val route = entry.toRoute<IdentityDetail>()
            IdentityDetailScreen(route.identityIdHex, navController)
        }

        composable<RegistrationProgress> { entry ->
            val route = entry.toRoute<RegistrationProgress>()
            RegistrationProgressScreen(route.walletIdHex, route.identityIndex, navController)
        }

        composable<RegisterName> { entry ->
            val route = entry.toRoute<RegisterName>()
            RegisterNameScreen(route.identityIdHex, navController)
        }

        composable<SelectMainName> { entry ->
            val route = entry.toRoute<SelectMainName>()
            SelectMainNameScreen(route.identityIdHex, navController)
        }

        composable<KeysList> { entry ->
            val route = entry.toRoute<KeysList>()
            KeysListScreen(route.identityIdHex, navController)
        }

        composable<KeyDetail> { entry ->
            val route = entry.toRoute<KeyDetail>()
            KeyDetailScreen(route.identityIdHex, route.keyId, navController)
        }

        composable<AddIdentityKey> { entry ->
            val route = entry.toRoute<AddIdentityKey>()
            AddIdentityKeyScreen(route.identityIdHex, navController)
        }

        // ── Credits graph ──────────────────────────────────────────────

        composable<TopUpIdentity> { entry ->
            TopUpIdentityScreen(entry.toRoute<TopUpIdentity>().identityIdHex, navController)
        }

        composable<TopUpIdentityFromCore> { entry ->
            TopUpIdentityFromCoreScreen(
                entry.toRoute<TopUpIdentityFromCore>().identityIdHex,
                navController,
            )
        }

        composable<TransferCredits> { entry ->
            TransferCreditsScreen(entry.toRoute<TransferCredits>().identityIdHex, navController)
        }

        composable<TransferToAddress> { entry ->
            TransferIdentityToAddressScreen(
                entry.toRoute<TransferToAddress>().identityIdHex,
                navController,
            )
        }

        composable<WithdrawCredits> { entry ->
            WithdrawCreditsScreen(entry.toRoute<WithdrawCredits>().identityIdHex, navController)
        }

        // ── State-transition catalog ───────────────────────────────────

        composable<StateTransitions> { StateTransitionsScreen(navController) }

        composable<TransitionCategoryRoute> { entry ->
            TransitionCategoryScreen(entry.toRoute<TransitionCategoryRoute>().categoryName, navController)
        }

        composable<TransitionDetailRoute> { entry ->
            TransitionDetailScreen(entry.toRoute<TransitionDetailRoute>().transitionKey, navController)
        }

        // ── Asset-lock funding graph ───────────────────────────────────

        composable<FundFromAssetLock> { entry ->
            val route = entry.toRoute<FundFromAssetLock>()
            FundFromAssetLockScreen(
                walletIdHex = route.walletIdHex,
                navController = navController,
                resumeOutPointHex = route.resumeOutPointHex.takeIf { it.isNotEmpty() },
            )
        }

        composable<TransferPlatformAddress> { entry ->
            TransferPlatformAddressScreen(
                entry.toRoute<TransferPlatformAddress>().walletIdHex,
                navController,
            )
        }

        composable<WithdrawPlatformAddress> { entry ->
            WithdrawPlatformAddressScreen(
                entry.toRoute<WithdrawPlatformAddress>().walletIdHex,
                navController,
            )
        }

        composable<AddressFundProgress> { entry ->
            val route = entry.toRoute<AddressFundProgress>()
            AddressFundProgressScreen(
                route.walletIdHex,
                route.platformAccountIndex,
                route.recipientHashHex,
                navController,
            )
        }

        // ── Shielded funding graph ─────────────────────────────────────

        composable<SeedShieldedPool> { entry ->
            SeedShieldedPoolScreen(entry.toRoute<SeedShieldedPool>().walletIdHex, navController)
        }

        composable<ShieldedFund> { entry ->
            ShieldedFundScreen(entry.toRoute<ShieldedFund>().walletIdHex, navController)
        }

        composable<ShieldedFundProgress> { entry ->
            val route = entry.toRoute<ShieldedFundProgress>()
            ShieldedFundProgressScreen(route.walletIdHex, route.recipientRaw43Hex, navController)
        }

        composable<ShieldedActivity> { entry ->
            ShieldedActivityScreen(entry.toRoute<ShieldedActivity>().walletIdHex, navController)
        }

        // ── Contracts graph ────────────────────────────────────────────

        composable<ContractsHome> { ContractsHomeScreen(navController) }

        composable<LocalContracts> { LocalDataContractsScreen(navController) }

        composable<RegisterContractSource> { entry ->
            val route = entry.toRoute<RegisterContractSource>()
            RegisterContractSourceScreen(
                identityIdHex = route.identityIdHex,
                navController = navController,
            )
        }

        composable<ContractDetail> { entry ->
            val route = entry.toRoute<ContractDetail>()
            DataContractDetailsScreen(route.contractIdHex, navController)
        }

        composable<DocumentTypeDetail> { entry ->
            val route = entry.toRoute<DocumentTypeDetail>()
            DocumentTypeDetailsScreen(route.contractIdHex, route.typeName, navController)
        }

        composable<Documents> { entry ->
            val route = entry.toRoute<Documents>()
            DocumentsScreen(route.contractIdHex, route.typeName, navController)
        }

        composable<NewDocument> { entry ->
            val route = entry.toRoute<NewDocument>()
            CreateDocumentScreen(route.contractIdHex, route.typeName, navController)
        }

        composable<DocumentFields> { entry ->
            val route = entry.toRoute<DocumentFields>()
            DocumentFieldsScreen(route.documentJson, navController)
        }

        composable<DocumentWithPrice> { entry ->
            val route = entry.toRoute<DocumentWithPrice>()
            DocumentWithPriceScreen(
                contractIdHex = route.contractIdHex,
                typeName = route.typeName,
                initialDocumentId = route.documentId,
                navController = navController,
            )
        }

        composable<DocumentActions> { entry ->
            val route = entry.toRoute<DocumentActions>()
            DocumentActionsScreen(
                contractIdHex = route.contractIdHex,
                typeName = route.typeName,
                initialDocumentId = route.documentId,
                navController = navController,
            )
        }

        composable<UpdateContract> { entry ->
            val route = entry.toRoute<UpdateContract>()
            UpdateContractScreen(
                prefillContractIdHex = route.contractIdHex,
                navController = navController,
            )
        }

        composable<CountDocuments> { entry ->
            CountDocumentsScreen(entry.toRoute<CountDocuments>(), navController)
        }

        composable<SumAverageDocuments> { entry ->
            SumAverageDocumentsScreen(entry.toRoute<SumAverageDocuments>(), navController)
        }

        composable<QueriesList> { QueriesListScreen(navController) }

        composable<QueryDetail> { entry ->
            val route = entry.toRoute<QueryDetail>()
            QueryDetailScreen(route.queryName, navController)
        }

        composable<GroveDbPathElements> { GroveDBPathElementsScreen(navController) }

        // ── Tokens graph (hosted under the Contracts tab) ──────────────

        composable<TokensHome> { TokensScreen(navController) }

        composable<TokenSearch> { TokenSearchScreen(navController) }

        composable<QuickBasicToken> { QuickBasicTokenScreen(navController) }

        composable<TokenDetail> { entry ->
            val route = entry.toRoute<TokenDetail>()
            TokenDetailsScreen(route.tokenIdHex, navController)
        }

        composable<TokenActionPermissions> { entry ->
            val route = entry.toRoute<TokenActionPermissions>()
            TokenActionPermissionsScreen(
                tokenIdHex = route.tokenIdHex,
                pinnedIdentityIdHex = route.identityIdHex,
                navController = navController,
            )
        }

        composable<TokenAction> { entry ->
            TokenActionScreen(entry.toRoute<TokenAction>(), navController)
        }

        composable<PendingGroupActions> { entry ->
            val route = entry.toRoute<PendingGroupActions>()
            PendingGroupActionsScreen(
                tokenIdHex = route.tokenIdHex,
                identityIdHex = route.identityIdHex,
                navController = navController,
            )
        }

        composable<CoSignProposal> { entry ->
            CoSignProposalScreen(entry.toRoute<CoSignProposal>(), navController)
        }

        // ── DashPay graph ──────────────────────────────────────────────

        composable<DashPayHome> { DashPayTabScreen(navController) }

        composable<DashPayContacts> { entry ->
            ContactsScreen(entry.toRoute<DashPayContacts>().identityIdHex, navController)
        }

        composable<DashPayRequests> { entry ->
            ContactRequestsScreen(entry.toRoute<DashPayRequests>().identityIdHex, navController)
        }

        composable<DashPayAddContact> { entry ->
            AddContactScreen(entry.toRoute<DashPayAddContact>().identityIdHex, navController)
        }

        composable<DashPayContactDetail> { entry ->
            val route = entry.toRoute<DashPayContactDetail>()
            ContactDetailScreen(route.identityIdHex, route.contactIdHex, navController)
        }

        composable<DashPayProfile> { entry ->
            DashPayProfileScreen(entry.toRoute<DashPayProfile>().identityIdHex, navController)
        }

        composable<DashPayIgnored> { entry ->
            IgnoredContactsScreen(entry.toRoute<DashPayIgnored>().identityIdHex, navController)
        }

        composable<DashPayHidden> { entry ->
            HiddenContactsScreen(entry.toRoute<DashPayHidden>().ownerIdentityIdHex, navController)
        }

        // ── Diagnostics graph ──────────────────────────────────────────

        composable<AddressQueries> { AddressQueriesScreen(navController) }

        composable<BannedAddresses> { BannedAddressesScreen(navController) }

        composable<ContestDetail> { entry ->
            val route = entry.toRoute<ContestDetail>()
            ContestDetailScreen(route.contestName, route.identityIdHex, navController)
        }

        composable<GroupDetail> { entry ->
            val route = entry.toRoute<GroupDetail>()
            GroupDetailScreen(route.contractIdHex, route.position, navController)
        }

        composable<Diagnostics> { DiagnosticsScreen(navController) }

        composable<WalletMemoryExplorer> { WalletMemoryExplorerScreen(navController) }

        composable<KeystoreExplorer> { KeystoreExplorerScreen(navController) }

        // ── Storage explorer graph ─────────────────────────────────────

        composable<StorageExplorer> { StorageExplorerScreen(navController) }

        composable<StorageModelList> { entry ->
            val route = entry.toRoute<StorageModelList>()
            StorageModelListScreen(route.modelName, navController)
        }

        composable<StorageRecordDetail> { entry ->
            val route = entry.toRoute<StorageRecordDetail>()
            StorageRecordDetailScreen(route.modelName, route.rowId, navController)
        }

        // ── Settings graph ─────────────────────────────────────────────

        composable<SettingsHome> { SettingsScreen(navController) }

        composable<DataManagement> { DataManagementScreen(navController) }
    }
}
