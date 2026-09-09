#!/usr/bin/env python3
"""Tests for freeze_schema_models.py.

Run from anywhere inside the repository:

    python3 -m unittest discover -s packages/swift-sdk/scripts -p 'test_*.py'

The closure tests exercise the real `FREEZES` table against the real
repository history, so they need the commits the table names to be fetched.
"""

import dataclasses
import os
import sys
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import freeze_schema_models as gen  # noqa: E402

ROOT = gen.repo_root()


def without(freezes, *, models=(), value_types=()):
    """`freezes` with the named models and value types dropped from every row."""
    return [
        dataclasses.replace(
            freeze,
            models=tuple(m for m in freeze.models if m not in models),
            value_types=tuple(v for v in freeze.value_types if v not in value_types),
        )
        for freeze in freezes
    ]


class ClosureTests(unittest.TestCase):
    """An incomplete freeze must be refused, never generated or passed by --check.

    A frozen model whose relationship target or stored value type is not
    frozen under the same schema compiles against the live type, so the
    released checksum silently follows the live type's next change.
    """

    def render(self, freezes):
        with mock.patch.object(gen, "FREEZES", freezes):
            return gen.render_all(ROOT)

    def refused(self, freezes):
        with self.assertRaises(SystemExit) as caught:
            self.render(freezes)
        return str(caught.exception)

    def test_the_committed_table_is_closed(self):
        files = self.render(gen.FREEZES)
        self.assertEqual(len(files), 37)

    def test_omitting_a_relationship_target_is_refused(self):
        # `PersistentTransaction.outputs` is a relationship to `PersistentTxo`.
        message = self.refused(without(gen.FREEZES, models={"PersistentTxo"}))
        self.assertIn("DashSchemaV1.PersistentTransaction stores PersistentTxo", message)
        self.assertIn("DashSchemaV1.PersistentTxo", message)

    def test_omitting_a_stored_value_type_is_refused(self):
        # `PersistentToken.freezeRules` is a `ChangeControlRules?` stored inline.
        message = self.refused(without(gen.FREEZES, value_types={"ChangeControlRules"}))
        self.assertIn("DashSchemaV1.PersistentToken stores ChangeControlRules", message)

    def test_omitting_a_transitively_stored_value_type_is_refused(self):
        # `DistributionEvent` is reached only through
        # `TokenPreProgrammedDistribution.distributionSchedule`.
        message = self.refused(without(gen.FREEZES, value_types={"DistributionEvent"}))
        self.assertIn(
            "DashSchemaV1.TokenPreProgrammedDistribution stores DistributionEvent",
            message,
        )


class RegistrationTests(unittest.TestCase):
    """What DashModelContainer.swift registers must be what FREEZES produces."""

    FROZEN = {("DashSchemaV1", "PersistentWallet")}
    LIVE = {"PersistentWallet", "PersistentTrackedMasternode"}

    def problems(self, source):
        return gen.registration_problems(source.splitlines(), self.FROZEN, self.LIVE)

    def test_a_frozen_graph_registered_by_the_live_list_only_is_clean(self):
        source = """
        enum DashModelContainer {
            static var v1ModelTypes: [any PersistentModel.Type] {
                [DashSchemaV1.PersistentWallet.self]
            }
            public static var modelTypes: [any PersistentModel.Type] {
                [
                    PersistentWallet.self,
                    PersistentTrackedMasternode.self
                ]
            }
        }
        """
        self.assertEqual(self.problems(source), [])

    def test_a_registered_type_the_table_does_not_freeze_is_refused(self):
        source = """
        static var v2ModelTypes: [any PersistentModel.Type] {
            [DashSchemaV1.PersistentWallet.self, DashSchemaV2.PersistentTrackedMasternode.self]
        }
        public static var modelTypes: [any PersistentModel.Type] {
            [PersistentWallet.self, PersistentTrackedMasternode.self]
        }
        """
        [problem] = self.problems(source)
        self.assertIn("DashSchemaV2.PersistentTrackedMasternode", problem)

    def test_a_live_model_in_a_released_list_is_refused(self):
        source = """
        static var v2ModelTypes: [any PersistentModel.Type] {
            [DashSchemaV1.PersistentWallet.self, PersistentTrackedMasternode.self]
        }
        public static var modelTypes: [any PersistentModel.Type] {
            [PersistentWallet.self, PersistentTrackedMasternode.self]
        }
        """
        [problem] = self.problems(source)
        self.assertIn("live PersistentTrackedMasternode", problem)

    def test_a_released_version_aliasing_the_live_list_is_refused(self):
        source = """
        static var v1ModelTypes: [any PersistentModel.Type] {
            [DashSchemaV1.PersistentWallet.self]
        }
        public static var modelTypes: [any PersistentModel.Type] {
            [PersistentWallet.self, PersistentTrackedMasternode.self]
        }
        public enum DashSchemaV1: VersionedSchema {
            public static var models: [any PersistentModel.Type] {
                DashModelContainer.v1ModelTypes
            }
        }
        public enum DashSchemaV2: VersionedSchema {
            public static var models: [any PersistentModel.Type] {
                DashModelContainer.modelTypes
            }
        }
        public enum DashSchemaV3: VersionedSchema {
            public static var models: [any PersistentModel.Type] {
                DashModelContainer.modelTypes
            }
        }
        """
        [problem] = self.problems(source)
        self.assertIn(
            "line 15 uses the live model list `modelTypes` outside DashSchemaV3", problem
        )

    def test_a_member_access_on_a_schema_is_not_a_registration(self):
        source = """
        static let oldest = DashSchemaV1.versionIdentifier
        static var v1ModelTypes: [any PersistentModel.Type] {
            [DashSchemaV1.PersistentWallet.self]
        }
        public static var modelTypes: [any PersistentModel.Type] {
            [PersistentWallet.self, PersistentTrackedMasternode.self]
        }
        """
        self.assertEqual(self.problems(source), [])

    def test_the_committed_container_registers_exactly_the_table(self):
        with open(os.path.join(ROOT, gen.CONTAINER_FILE), encoding="utf-8") as f:
            lines = f.read().splitlines()
        frozen = {(f.schema, m) for f in gen.FREEZES for m in f.models}
        live = {
            name[: -len(".swift")]
            for name in os.listdir(os.path.join(ROOT, gen.MODELS_DIR))
        }
        self.assertEqual(gen.registration_problems(lines, frozen, live), [])

    def test_a_frozen_model_nothing_registers_is_refused(self):
        source = """
        public static var modelTypes: [any PersistentModel.Type] {
            [PersistentWallet.self, PersistentTrackedMasternode.self]
        }
        """
        [problem] = self.problems(source)
        self.assertIn("DashSchemaV1.PersistentWallet", problem)


class StoredTypeTests(unittest.TestCase):
    """Only stored declarations feed the entity hash, so only they are closed over."""

    def names(self, text):
        return gen.stored_type_names(text.strip("\n").splitlines())

    def test_stored_properties_and_relationships_count(self):
        body = """
        @Model
        final class PersistentThing {
            @Attribute(.unique) var id: Data
            var rules: ChangeControlRules?
            var byName: [String: TokenLocalization]?
            @Relationship(deleteRule: .cascade, inverse: \\PersistentTxo.thing)
            var outputs: [PersistentTxo] = []
            var owner: PersistentWallet?
        }
        """
        self.assertEqual(
            self.names(body),
            {"ChangeControlRules", "TokenLocalization", "PersistentTxo", "PersistentWallet"},
        )

    def test_computed_properties_methods_and_nested_types_do_not_count(self):
        body = """
        @Model
        final class PersistentThing {
            var networkRaw: UInt32
            var network: Network {
                Network(rawValue: networkRaw) ?? .mainnet
            }
            var kind: Kind
            enum Kind: String, Codable {
                case a, b
            }
            init(from info: TokenInfo) {
                let helper: Helper = Helper()
                networkRaw = 0
            }
            func convert(to target: Target) -> Result { fatalError() }
        }
        """
        self.assertEqual(self.names(body), set())

    def test_access_modifiers_and_inferred_types_still_count(self):
        body = """
        @Model
        final class PersistentThing {
            private var rules: ChangeControlRules?
            private(set) var localization: TokenLocalization?
            var tradeMode = TokenTradeMode.notTradeable
            let events = [DistributionEvent]()
            var stamp = Date()
            static let shared: Registry = Registry()
            @Attribute(.transformable(by: ColorTransformer.self)) var color: ColorBox
            var observed: Counter {
                didSet { print("changed") }
            }
        }
        """
        self.assertEqual(
            self.names(body),
            {
                "ChangeControlRules",
                "TokenLocalization",
                "TokenTradeMode",
                "DistributionEvent",
                "ColorBox",
                "Counter",
            },
        )

    def test_fields_of_a_nested_type_are_walked(self):
        body = """
        @Model
        final class PersistentThing {
            var meta: Meta?
            struct Meta: Codable {
                var rules: ChangeControlRules
                var kind: Kind
                enum Kind: String, Codable {
                    case a(DistributionEvent)
                }
            }
        }
        """
        self.assertEqual(self.names(body), {"ChangeControlRules", "DistributionEvent"})

    def test_transient_comments_strings_and_split_declarations(self):
        body = """
        @Model
        final class PersistentThing {
            @Transient var cache: LiveCache? = nil
            var id: Data // Base58 identity id, see IdentityEntry
            @Relationship(deleteRule: .cascade)
            var outputs:
                [PersistentTxo] = []
            init() { let brace = "{"; _ = brace }
            var late: TokenTradeMode
        }
        """
        self.assertEqual(self.names(body), {"PersistentTxo", "TokenTradeMode"})

    def test_a_type_declared_in_the_models_own_extension_is_not_missing(self):
        body = """
        final class PersistentThing {
            var kind: Kind
        }
        """.strip("\n").splitlines()
        bodies = {("DashSchemaV1", "PersistentThing"): (body, {"Kind"})}
        frozen = {"DashSchemaV1": {"PersistentThing"}}
        self.assertEqual(gen.closure_problems(frozen, bodies, {"DashSchemaV1": set()}), [])
        bodies = {("DashSchemaV1", "PersistentThing"): (body, set())}
        [problem] = gen.closure_problems(frozen, bodies, {"DashSchemaV1": set()})
        self.assertIn("stores Kind", problem)

    def test_enum_payloads_count_but_raw_values_do_not(self):
        body = """
        enum Distribution: String, Codable {
            case none = "None"
            case perpetual(TokenPerpetualDistribution)
            case scheduled(events: [DistributionEvent])
        }
        """
        self.assertEqual(
            self.names(body), {"TokenPerpetualDistribution", "DistributionEvent"}
        )


if __name__ == "__main__":
    unittest.main()
