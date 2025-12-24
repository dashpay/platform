import SwiftUI

struct TokenDetailsView: View {
    let token: PersistentToken
    @Environment(\.dismiss) var dismiss
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                // Basic Information
                basicInfoSection
                
                // Localization
                if let localizations = token.localizations, !localizations.isEmpty {
                    localizationSection(localizations)
                }
                
                // Supply Information
                supplySection
                
                // Token Features
                featuresSection
                
                // History Keeping Rules
                historyKeepingSection
                
                // Control Rules
                controlRulesSection
                
                // Distribution Rules
                if token.perpetualDistribution != nil || token.preProgrammedDistribution != nil {
                    distributionSection
                }
                
                // Trade Mode
                tradeModeSection
            }
            .padding()
        }
        .navigationTitle(token.getPluralForm(languageCode: "en") ?? token.name)
        .navigationBarTitleDisplayMode(.inline)
    }
    
    // MARK: - Section Views
    
    @ViewBuilder
    private var basicInfoSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "Basic Information")
            
            InfoRow(label: "Name:", value: token.name)
            // Remove symbol as it doesn't exist in PersistentToken
            InfoRow(label: "Description:", value: token.tokenDescription ?? "No description")
            InfoRow(label: "Position:", value: "\(token.position)")
            InfoRow(label: "Decimals:", value: "\(token.decimals)")
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    @ViewBuilder
    private func localizationSection(_ localizations: [String: TokenLocalization]) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "Localizations")
            
            ForEach(localizations.sorted(by: { $0.key < $1.key }), id: \.key) { languageCode, localization in
                VStack(alignment: .leading, spacing: 8) {
                    Text(languageCode.uppercased())
                        .font(.caption)
                        .fontWeight(.semibold)
                        .foregroundColor(.secondary)
                    
                    HStack {
                        VStack(alignment: .leading) {
                            Text("Singular: \(localization.singularForm)")
                                .font(.subheadline)
                            Text("Plural: \(localization.pluralForm)")
                                .font(.subheadline)
                        }
                        Spacer()
                    }
                    
                    if let desc = localization.description {
                        Text(desc)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    if languageCode != localizations.sorted(by: { $0.key < $1.key }).last?.key {
                        Divider()
                    }
                }
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    @ViewBuilder
    private var supplySection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "Supply Information")
            
            InfoRow(label: "Base Supply:", value: token.formattedBaseSupply)
            
            if let maxSupply = token.maxSupply {
                InfoRow(label: "Max Supply:", value: formatTokenAmount(maxSupply))
            } else {
                InfoRow(label: "Max Supply:", value: "Unlimited")
            }
            
            InfoRow(label: "Max Supply Changeable:", value: token.maxSupplyChangeRules != nil ? "Yes" : "No")
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    @ViewBuilder
    private var featuresSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "Token Features")
            
            VStack(alignment: .leading, spacing: 8) {
                TokenFeatureRow(label: "Can be minted", isEnabled: token.manualMintingRules != nil)
                TokenFeatureRow(label: "Can be burned", isEnabled: token.manualBurningRules != nil)
                TokenFeatureRow(label: "Can be frozen", isEnabled: token.freezeRules != nil)
                TokenFeatureRow(label: "Can be unfrozen", isEnabled: token.unfreezeRules != nil)
                TokenFeatureRow(label: "Can destroy frozen funds", isEnabled: token.destroyFrozenFundsRules != nil)
                TokenFeatureRow(label: "Transfer to frozen allowed", isEnabled: token.allowTransferToFrozenBalance)
                TokenFeatureRow(label: "Emergency action available", isEnabled: token.emergencyActionRules != nil)
                TokenFeatureRow(label: "Started as paused", isEnabled: token.isPaused)
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    @ViewBuilder
    private var historyKeepingSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "History Keeping")
            
            VStack(alignment: .leading, spacing: 8) {
                TokenFeatureRow(label: "Transfer history", isEnabled: token.keepsTransferHistory)
                TokenFeatureRow(label: "Freezing history", isEnabled: token.keepsFreezingHistory)
                TokenFeatureRow(label: "Minting history", isEnabled: token.keepsMintingHistory)
                TokenFeatureRow(label: "Burning history", isEnabled: token.keepsBurningHistory)
                TokenFeatureRow(label: "Direct pricing history", isEnabled: token.keepsDirectPricingHistory)
                TokenFeatureRow(label: "Direct purchase history", isEnabled: token.keepsDirectPurchaseHistory)
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    @ViewBuilder
    private var controlRulesSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "Control Rules")
            
            VStack(alignment: .leading, spacing: 12) {
                if let rule = token.conventionsChangeRules {
                    ControlRuleView(title: "Conventions", rule: rule)
                }
                if let rule = token.maxSupplyChangeRules {
                    ControlRuleView(title: "Max Supply", rule: rule)
                }
                if let rule = token.manualMintingRules {
                    ControlRuleView(title: "Manual Minting", rule: rule)
                }
                if let rule = token.manualBurningRules {
                    ControlRuleView(title: "Manual Burning", rule: rule)
                }
                if let rule = token.freezeRules {
                    ControlRuleView(title: "Freeze", rule: rule)
                }
                if let rule = token.unfreezeRules {
                    ControlRuleView(title: "Unfreeze", rule: rule)
                }
                if let rule = token.destroyFrozenFundsRules {
                    ControlRuleView(title: "Destroy Frozen Funds", rule: rule)
                }
                if let rule = token.emergencyActionRules {
                    ControlRuleView(title: "Emergency Action", rule: rule)
                }
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    @ViewBuilder
    private var distributionSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "Distribution")
            
            if let perpetual = token.perpetualDistribution {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Perpetual Distribution")
                        .font(.subheadline)
                        .fontWeight(.semibold)
                    
                    InfoRow(label: "Enabled:", value: perpetual.enabled ? "Yes" : "No")
                    InfoRow(label: "Recipient:", value: perpetual.distributionRecipient)
                    
                    // Parse and display distribution type details
                    if let typeData = perpetual.distributionType.data(using: .utf8),
                       let typeJson = try? JSONSerialization.jsonObject(with: typeData) as? [String: Any],
                       let timeBased = typeJson["TimeBasedDistribution"] as? [String: Any] {
                        
                        if let interval = timeBased["interval"] as? Int {
                            let hours = interval / 3600000
                            InfoRow(label: "Interval:", value: "\(hours) hour\(hours != 1 ? "s" : "")")
                        }
                        
                        if let function = timeBased["function"] as? [String: Any],
                           let fixedAmount = function["FixedAmount"] as? [String: Any],
                           let amount = fixedAmount["amount"] as? Int {
                            InfoRow(label: "Amount per interval:", value: "\(amount)")
                        }
                    }
                    
                    if let lastTime = perpetual.lastDistributionTime {
                        InfoRow(label: "Last distribution:", value: lastTime, style: .relative)
                    }
                    if let nextTime = perpetual.nextDistributionTime {
                        InfoRow(label: "Next distribution:", value: nextTime, style: .relative)
                    }
                }
            }
            
            if let preProgrammed = token.preProgrammedDistribution {
                Divider()
                VStack(alignment: .leading, spacing: 8) {
                    Text("Pre-programmed Distribution")
                        .font(.subheadline)
                        .fontWeight(.semibold)
                    
                    InfoRow(label: "Active:", value: preProgrammed.isActive ? "Yes" : "No")
                    InfoRow(label: "Events:", value: "\(preProgrammed.distributionSchedule.count)")
                    InfoRow(label: "Total distributed:", value: formatTokenAmount(preProgrammed.totalDistributed))
                    InfoRow(label: "Remaining:", value: formatTokenAmount(preProgrammed.remainingToDistribute))
                }
            }
            
            // New tokens destination
            if let destinationId = token.newTokensDestinationIdentityBase58 {
                Divider()
                VStack(alignment: .leading, spacing: 8) {
                    Text("New Tokens Configuration")
                        .font(.subheadline)
                        .fontWeight(.semibold)
                    
                    InfoRow(label: "Destination Identity:", value: destinationId)
                    InfoRow(label: "Allow choosing destination:", value: token.mintingAllowChoosingDestination ? "Yes" : "No")
                }
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    @ViewBuilder
    private var tradeModeSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionHeader(title: "Trade Mode")
            
            InfoRow(label: "Trade Mode:", value: token.tradeMode.displayName)
            
            if let changeRules = token.tradeModeChangeRules {
                ControlRuleView(title: "Trade Mode Change", rule: changeRules)
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    // MARK: - Helper Methods
    
    private func formatTokenAmount(_ amount: String) -> String {
        guard let value = Double(amount) else { return amount }
        let divisor = pow(10.0, Double(token.decimals))
        let actualAmount = value / divisor
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = token.decimals
        formatter.minimumFractionDigits = 0
        return formatter.string(from: NSNumber(value: actualAmount)) ?? amount
    }
    
    private func formatDuration(_ seconds: Int64) -> String {
        let hours = seconds / 3600
        let minutes = (seconds % 3600) / 60
        let secs = seconds % 60
        
        if hours > 0 {
            return "\(hours)h \(minutes)m \(secs)s"
        } else if minutes > 0 {
            return "\(minutes)m \(secs)s"
        } else {
            return "\(secs)s"
        }
    }
}

// MARK: - Helper Views

struct SectionHeader: View {
    let title: String
    
    var body: some View {
        Text(title)
            .font(.headline)
            .foregroundColor(.primary)
    }
}

struct TokenFeatureRow: View {
    let label: String
    let isEnabled: Bool
    
    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Image(systemName: isEnabled ? "checkmark.circle.fill" : "xmark.circle")
                .foregroundColor(isEnabled ? .green : .gray)
        }
    }
}

struct ControlRuleView: View {
    let title: String
    let rule: ChangeControlRules
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.subheadline)
                .fontWeight(.medium)
            
            Text("Authorized: \(rule.authorizedToMakeChange)")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Text("Admin: \(rule.adminActionTakers)")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
}