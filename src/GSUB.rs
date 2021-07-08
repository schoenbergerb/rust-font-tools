use crate::layout::common::*;
use crate::layout::gsub1::SingleSubst;
use crate::layout::gsub1::SingleSubstInternal;
use crate::layout::gsub2::MultipleSubst;
use crate::layout::gsub3::AlternateSubst;
use crate::layout::gsub4::LigatureSubst;

use otspec::types::*;
use otspec::Counted;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use otspec_macros::{tables, Deserialize, Serialize};
use std::convert::TryInto;

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug, Clone, Deserialize)]
pub struct gsubcoreincoming {
    pub majorVersion: uint16,
    pub minorVersion: uint16,
    pub scriptList: Offset16<ScriptList>,
    pub featureList: Offset16<FeatureList>,
    pub lookupList: Offset16<SubstLookupListIncoming>,
}

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug, Serialize)]
pub struct gsubcoreoutgoing {
    pub majorVersion: uint16,
    pub minorVersion: uint16,
    pub scriptList: Offset16<ScriptList>,
    pub featureList: Offset16<FeatureList>,
    pub lookupList: Offset16<SubstLookupListOutgoing>,
}

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug, Clone, Deserialize)]
pub struct SubstLookupListIncoming {
    #[serde(offset_base)]
    #[serde(with = "Counted")]
    pub lookups: VecOffset16<SubstLookup>,
}

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug)]
pub struct SubstLookupListOutgoing {
    lookups: VecOffset16<LookupInternal>,
}

#[automatically_derived]
impl otspec::Serialize for SubstLookupListOutgoing {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = otspec::offsetmanager::resolve_offsets(self);
        self.to_bytes_shallow(data)?;
        otspec::offsetmanager::resolve_offsets_and_serialize(obj, data, false)?;
        Ok(())
    }
    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = self;
        data.put(self.lookups.0.len() as uint16)?;
        self.lookups.0.to_bytes_shallow(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 + 2 * self.lookups.0.len()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        let mut v: Vec<&dyn OffsetMarkerTrait> = Vec::new();
        v.extend(self.lookups.offset_fields());
        v
    }
}

/// A general substitution lookup rule, of whatever type
#[derive(Debug, PartialEq, Clone)]
pub struct SubstLookup {
    /// Lookup flags
    pub flags: LookupFlags,
    /// The mark filtering set index in the `GDEF` table.
    pub mark_filtering_set: Option<uint16>,
    /// The concrete substitution rule.
    pub substitution: Substitution,
}

#[derive(Debug)]
struct LookupInternal {
    pub lookupType: uint16,
    pub flags: LookupFlags,
    pub subtables: Vec<Box<dyn OffsetMarkerTrait>>,
    pub mark_filtering_set: Option<uint16>,
}

impl otspec::Serialize for LookupInternal {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = otspec::offsetmanager::resolve_offsets(self);
        self.to_bytes_shallow(data)?;
        otspec::offsetmanager::resolve_offsets_and_serialize(obj, data, false)?;
        Ok(())
    }
    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = self;
        obj.lookupType.to_bytes(data)?;
        obj.flags.to_bytes(data)?;
        (obj.subtables.len() as uint16).to_bytes(data)?;
        for st in &obj.subtables {
            st.to_bytes_shallow(data)?;
        }
        obj.mark_filtering_set.to_bytes(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        self.lookupType.ot_binary_size()
            + self.flags.ot_binary_size()
            + 2
            + 2 * self.subtables.len()
            + self.mark_filtering_set.ot_binary_size()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.subtables.iter().map(|x| x.as_ref()).collect()
    }
}

impl Clone for LookupInternal {
    fn clone(&self) -> Self {
        panic!("Can't clone this")
    }
}

impl SubstLookup {
    fn lookup_type(&self) -> u16 {
        match self.substitution {
            Substitution::Single(_) => 1,
            Substitution::Multiple(_) => 2,
            Substitution::Alternate(_) => 3,
            Substitution::Ligature(_) => 4,
            Substitution::Contextual => 5,
            Substitution::ChainedContextual => 6,
            Substitution::Extension => 7,
            Substitution::ReverseChaining => 8,
        }
    }
}
/// A container which represents a generic substitution rule
///
/// Each rule is expressed as a vector of subtables.
#[derive(Debug, PartialEq, Clone)]
pub enum Substitution {
    /// Contains a single substitution rule.
    Single(Vec<SingleSubst>),
    /// Contains a multiple substitution rule.
    Multiple(Vec<MultipleSubst>),
    /// Contains an alternate substitution rule.
    Alternate(Vec<AlternateSubst>),
    /// Contains a ligature substitution rule.
    Ligature(Vec<LigatureSubst>),
    /// Contains a contextual substitution rule.
    Contextual,
    /// Contains a chained contextual substitution rule.
    ChainedContextual,
    /// Contains an extension subtable.
    Extension,
    /// Contains a reverse chaining single substitution rule.
    ReverseChaining,
}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::upper_case_acronyms)]
/// The Glyph Substitution table
pub struct GSUB {
    /// A list of substitution lookups
    pub lookups: Vec<SubstLookup>,
    /// A mapping between script tags and `Script` tables.
    pub scripts: ScriptList,
    /// The association between feature tags and the list of indices into the
    /// lookup table used to process this feature, together with any feature parameters.
    pub features: Vec<(Tag, Vec<usize>, Option<FeatureParams>)>,
}

impl Deserialize for GSUB {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let core: gsubcoreincoming = c.de()?;
        if core.minorVersion == 1 {
            let _featureVariationsOffset: uint16 = c.de()?;
        }
        let scripts: ScriptList = core
            .scriptList
            .link
            .ok_or_else(|| DeserializationError("Bad script list in GSUB table".to_string()))?;
        let lookups: Vec<SubstLookup> = core
            .lookupList
            .link
            .ok_or_else(|| DeserializationError("Bad lookup list in GSUB table".to_string()))?
            .lookups
            .try_into()?;
        let feature_records = core
            .featureList
            .link
            .ok_or_else(|| DeserializationError("Bad feature list in GSUB table".to_string()))?
            .featureRecords;
        let mut features = vec![];
        for f in feature_records.iter() {
            let tag = f.featureTag;
            let table = f
                .feature
                .link
                .as_ref()
                .ok_or_else(|| DeserializationError("Bad feature in GSUB table".to_string()))?;
            features.push((
                tag,
                table
                    .lookupListIndices
                    .iter()
                    .map(|x| *x as usize)
                    .collect(),
                None,
            ));
        }

        Ok(GSUB {
            lookups,
            scripts,
            features,
        })
    }
}

impl From<&GSUB> for gsubcoreoutgoing {
    fn from(val: &GSUB) -> Self {
        let substlookuplist: SubstLookupListOutgoing = SubstLookupListOutgoing {
            lookups: VecOffset16(val.lookups.iter().map(|x| Offset16::to(x.into())).collect()),
        };
        let featurelist: FeatureList = FeatureList {
            featureRecords: val
                .features
                .iter()
                .map(|f| {
                    let indices: Vec<uint16> = f.1.iter().map(|x| *x as uint16).collect();
                    FeatureRecord {
                        featureTag: f.0,
                        feature: Offset16::to(FeatureTable {
                            featureParamsOffset: 0,
                            lookupListIndices: indices,
                        }),
                    }
                })
                .collect(),
        };
        gsubcoreoutgoing {
            majorVersion: 1,
            minorVersion: 0,
            scriptList: Offset16::to(val.scripts.clone()),
            featureList: Offset16::to(featurelist),
            lookupList: Offset16::to(substlookuplist),
        }
    }
}

impl Deserialize for SubstLookup {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let lookup_type: uint16 = c.de()?;
        let lookup_flag: LookupFlags = c.de()?;
        let substitution = match lookup_type {
            1 => {
                let stuff: Counted<Offset16<SingleSubst>> = c.de()?;
                Substitution::Single(stuff.try_into()?)
            }
            2 => {
                let stuff: Counted<Offset16<MultipleSubst>> = c.de()?;
                Substitution::Multiple(stuff.try_into()?)
            }
            3 => {
                let stuff: Counted<Offset16<AlternateSubst>> = c.de()?;
                Substitution::Alternate(stuff.try_into()?)
            }
            4 => {
                let stuff: Counted<Offset16<LigatureSubst>> = c.de()?;
                Substitution::Ligature(stuff.try_into()?)
            }
            _ => {
                panic!("Bad lookup type: {}", lookup_type)
            }
        };
        c.pop();
        Ok(SubstLookup {
            flags: lookup_flag,
            mark_filtering_set: None,
            substitution,
        })
    }
}

impl<'a> From<&SubstLookup> for LookupInternal {
    fn from(val: &SubstLookup) -> Self {
        let subtables: Vec<Box<dyn OffsetMarkerTrait>> = match &val.substitution.clone() {
            Substitution::Single(subs) => {
                let mut v: Vec<Box<dyn OffsetMarkerTrait>> = vec![];
                for s in subs {
                    let si: SingleSubstInternal = s.into();
                    v.push(Box::new(Offset16::to(si)));
                }
                v
            }
            // Substitution::Multiple(subs) => subs.offset_fields(),
            // Substitution::Alternate(subs) => subs.offset_fields(),
            // Substitution::Ligature(subs) => subs.offset_fields(),
            _ => unimplemented!(),
        };

        LookupInternal {
            flags: val.flags,
            lookupType: val.lookup_type(),
            mark_filtering_set: val.mark_filtering_set,
            subtables,
        }
    }
}

impl Serialize for GSUB {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let gsc: gsubcoreoutgoing = self.into();
        gsc.to_bytes(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Offset;
    use otspec::offsetmanager::OffsetManager;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    macro_rules! hashmap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

    macro_rules! btreemap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }
    #[test]
    fn test_simple_gsub_de() {
        /* languagesystem DFLT dflt;
           lookup ssf1 { sub a by b; sub c by d; } ssf1;
           lookup ssf2 { sub A by a; sub B by a; sub C by a; } ssf2;
           lookup mult { sub i by f i; sub l by f l; } mult;
           lookup aalt {sub a from [b c d]; } aalt;

           feature sing { lookup ssf1; lookup ssf2; } sing;
           feature mult { lookup mult; } mult;
           feature alte { lookup aalt; } alte;
        */
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x24, 0x00, 0x58, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x61, 0x6c, 0x74, 0x65,
            0x00, 0x1a, 0x6c, 0x69, 0x67, 0x61, 0x00, 0x20, 0x6d, 0x75, 0x6c, 0x74, 0x00, 0x26,
            0x73, 0x69, 0x6e, 0x67, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x05, 0x00, 0x0c, 0x00, 0x22, 0x00, 0x40, 0x00, 0x66,
            0x00, 0x7e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x06,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x42, 0x00, 0x44, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x08, 0x00, 0x02, 0x00, 0x0c, 0x00, 0x03, 0x00, 0x42, 0x00, 0x42,
            0x00, 0x42, 0x00, 0x01, 0x00, 0x03, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x0a, 0x00, 0x02, 0x00, 0x12,
            0x00, 0x18, 0x00, 0x01, 0x00, 0x02, 0x00, 0x4a, 0x00, 0x4d, 0x00, 0x02, 0x00, 0x47,
            0x00, 0x4a, 0x00, 0x02, 0x00, 0x47, 0x00, 0x4d, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x08, 0x00, 0x01, 0x00, 0x2a, 0x00, 0x01, 0x00, 0x08, 0x00, 0x03, 0x00, 0x43,
            0x00, 0x44, 0x00, 0x45, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x01,
            0x00, 0x12, 0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x04, 0x00, 0x44, 0x00, 0x02,
            0x00, 0x43, 0x00, 0x01, 0x00, 0x01, 0x00, 0x42,
        ];
        let expected = GSUB {
            lookups: vec![
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Single(vec![SingleSubst {
                        mapping: btreemap!(66 => 67, 68 => 69),
                    }]),
                },
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Single(vec![SingleSubst {
                        mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
                    }]),
                },
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Multiple(vec![MultipleSubst {
                        mapping: btreemap!(77 => vec![71,77], 74 => vec![71,74]),
                    }]),
                },
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Alternate(vec![AlternateSubst {
                        mapping: btreemap!(66 => vec![67,68,69]),
                    }]),
                },
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Ligature(vec![LigatureSubst {
                        mapping: btreemap!(vec![66,67] => 68),
                    }]),
                },
            ],
            scripts: ScriptList {
                scripts: hashmap!(*b"DFLT" => Script {
                    default_language_system: Some(
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                0,
                                1,
                                2,
                                3,
                           ],
                        },
                    ),
                    language_systems: BTreeMap::new()
                }),
            },
            features: vec![
                (*b"alte", vec![3], None),
                (*b"liga", vec![4], None),
                (*b"mult", vec![2], None),
                (*b"sing", vec![0, 1], None),
            ],
        };
        let deserialized: GSUB = otspec::de::from_bytes(&binary_gsub).unwrap();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_very_simple_gsub_ser() {
        let gsub = GSUB {
            lookups: vec![SubstLookup {
                flags: LookupFlags::empty(),
                mark_filtering_set: None,
                substitution: Substitution::Single(vec![SingleSubst {
                    mapping: btreemap!(386 => 459),
                }]),
            }],
            scripts: ScriptList {
                scripts: hashmap!(*b"DFLT" => Script {
                    default_language_system: Some(
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                0,
                           ],
                        },
                    ),
                    language_systems: BTreeMap::new()
                }),
            },
            features: vec![(*b"liga", vec![0], None)],
        };

        let serialized = otspec::ser::to_bytes(&gsub).unwrap();

        assert_eq!(
            serialized,
            vec![
                /* 00 */ 0x00, 0x01, 0x00, 0x00, // version 1.0
                /* 04 */ 0x00, 0x0a, // offset to script list
                /* 06 */ 0x00, 0x1e, // offset to feature list
                /* 08 */ 0x00, 0x2c, // offset to lookup list
                /* 0a */ 0x00, 0x01, // ScriptList: script count
                /* 0c */ 0x44, 0x46, 0x4c, 0x54, // script tag
                /* 10 */ 0x00, 0x08, // script table offset: 0x08 + 0x0a = 0x12
                /* 12 */ 0x00,
                0x04, // ScriptTable: default langsys offset = 0x12 + 0x14 = 0x16
                /* 14 */ 0x00, 0x00, // langsys count
                /* 16 */ 0x00, 0x00, // LangSys table: lookupOrderOffset
                /* 18 */ 0xff, 0xff, // required feature index
                /* 1a */ 0x00, 0x01, // feature index count
                /* 1c */ 0x00, 0x00, // feature index = 0
                /* 1e */ 0x00, 0x01, // Feature list: Feature count = 1
                /* 20 */ 0x6c, 0x69, 0x67, 0x61, // Tag: liga
                /* 24 */ 0x00, 0x08, // Offset 1e + 08 = 26
                /* 26 */ 0x00, 0x00, // Feature table; featureParamsOffset
                /* 28 */ 0x00, 0x01, // lookupIndexCount
                /* 2a */ 0x00, 0x00, // lookup index = 0
                /* 2c */ 0x00, 0x01, // Lookup list: lookupCount = 1
                /* 2e */ 0x00, 0x04, // Offset to first lookup = 2c+04 = 30
                /* 30 */ 0x00, 0x01, // Lookup table Lookup type
                /* 32 */ 0x00, 0x00, // lookup flag
                /* 34 */ 0x00, 0x01, // subtable count
                /* 36 */ 0x00, 0x08, // subtable offset 30 + 08 = 38
                0x00, 0x01, 0x00, 0x06, 0x00, 0x49, 0x00, 0x01, 0x00, 0x01, 0x01, 0x82
            ]
        );
    }
}
