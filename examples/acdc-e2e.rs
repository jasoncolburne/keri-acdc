use std::collections::HashMap;

use cesride::{
    data::{dat, Value},
    Matter, Sadder,
};
use keri_acdc::{
    acdc,
    error::{Error, Result},
    keri::{self, KeriStore, KeySet},
};

struct Store {
    prefix: String,
    keys: HashMap<String, Vec<String>>,
    sads: HashMap<String, String>,
    attachments: HashMap<String, String>,
    acdcs: HashMap<String, Vec<String>>, // keys are "issued", "received"
    tels: HashMap<String, Vec<String>>,
    kels: HashMap<String, Vec<String>>,
}

impl Store {
    pub fn new(prefix: &str) -> Self {
        let keys = HashMap::new();
        let sads = HashMap::new();
        let attachments = HashMap::new();
        let mut acdcs = HashMap::new();
        acdcs.insert("issued".to_string(), vec![]);
        acdcs.insert("received".to_string(), vec![]);
        let tels = HashMap::new();
        let kels = HashMap::new();

        Store {
            prefix: prefix.to_string(),
            keys,
            sads,
            attachments,
            acdcs,
            tels,
            kels,
        }
    }

    fn insert_sad_internal(&mut self, sad: &str) -> Result<()> {
        let v: serde_json::Value = serde_json::from_str(sad)?;
        let value = Value::from(&v);
        let label = value["d"].to_string()?;

        self.sads.insert(label, sad.to_string());

        Ok(())
    }

    fn insert_event(&mut self, event: &str) -> Result<String> {
        let serder = cesride::Serder::new_with_raw(event.as_bytes())?;
        let said = serder.said()?;
        let attachments = &event[serder.raw().len()..];

        self.insert_sad_internal(&event[..serder.raw().len()])?;
        self.attachments
            .insert(said.clone(), attachments.to_string());

        Ok(said)
    }

    fn get_sad_and_attachments(&self, said: &str) -> Result<String> {
        if !self.sads.contains_key(said) || !self.attachments.contains_key(said) {
            return Err(Error::Value.into());
        }

        let sad = self.sads[said].clone();
        let atc = &self.attachments[said];
        Ok(sad + atc)
    }
}

impl keri::KeriStore for Store {
    fn prefix(&self) -> String {
        self.prefix.clone()
    }

    fn insert_keys(&mut self, pre: &str, keys: &keri::KeySet) -> Result<()> {
        if !self.keys.contains_key(pre) {
            self.keys.insert(pre.to_string(), vec![]);
        }

        let identifier_keys = self.keys.get_mut(pre).unwrap();

        let value = serde_json::to_string(keys)?;
        identifier_keys.push(value);

        Ok(())
    }

    fn insert_sad(&mut self, sad: &str) -> Result<()> {
        self.insert_sad_internal(sad)
    }

    fn insert_acdc(&mut self, acdc: &str, issued: bool) -> Result<()> {
        let creder = cesride::Creder::new_with_raw(acdc.as_bytes())?;
        let said = creder.said()?;
        let attachments = &acdc[creder.raw().len()..];

        let label = if issued { "issued" } else { "received" };

        let acdcs = self.acdcs.get_mut(label).unwrap();
        acdcs.push(said.clone());

        self.insert_sad_internal(&acdc[..creder.raw().len()])?;
        self.attachments.insert(said, attachments.to_string());

        Ok(())
    }

    fn insert_key_event(&mut self, pre: &str, event: &str) -> Result<()> {
        let said = self.insert_event(event)?;

        if !self.kels.contains_key(pre) {
            self.kels.insert(pre.to_string(), vec![]);
        }

        self.kels.get_mut(pre).unwrap().push(said);
        Ok(())
    }

    fn insert_transaction_event(&mut self, pre: &str, event: &str) -> Result<()> {
        let said = self.insert_event(event)?;

        if !self.tels.contains_key(pre) {
            self.tels.insert(pre.to_string(), vec![]);
        }

        self.tels.get_mut(pre).unwrap().push(said);

        Ok(())
    }

    fn get_current_keys(&self, pre: &str) -> Result<KeySet> {
        let keys = self.keys.get(pre).unwrap();

        if keys.len() < 2 {
            return Err(Error::Decoding.into());
        }

        let value = &keys[keys.len() - 2];
        let key_set: KeySet = serde_json::from_str(value)?;

        Ok(key_set)
    }

    fn get_next_keys(&self, pre: &str) -> Result<KeySet> {
        let keys = self.keys.get(pre).unwrap();

        if keys.is_empty() {
            return Err(Error::Decoding.into());
        }

        let value = &keys[keys.len() - 1];
        let key_set: KeySet = serde_json::from_str(value)?;

        Ok(key_set)
    }

    fn get_sad(&self, said: &str) -> Result<Value> {
        if !self.sads.contains_key(said) {
            return Err(Error::Value.into());
        }

        let v: serde_json::Value = serde_json::from_str(&self.sads[said])?;
        Ok(Value::from(&v))
    }

    fn get_acdc(&self, said: &str) -> Result<String> {
        self.get_sad_and_attachments(said)
    }

    fn get_key_event(&self, pre: &str, version: u32) -> Result<String> {
        if !self.kels.contains_key(pre) {
            return Err(Error::Value.into());
        }

        let kel = &self.kels[pre];
        if kel.len() < version as usize {
            return Err(Error::Value.into());
        }

        let said = &kel[version as usize];
        self.get_sad_and_attachments(said)
    }

    fn get_transaction_event(&self, pre: &str, version: u32) -> Result<String> {
        if !self.tels.contains_key(pre) {
            return Err(Error::Value.into());
        }

        let tel = &self.tels[pre];
        if tel.len() < version as usize {
            return Err(Error::Value.into());
        }

        let said = &tel[version as usize];
        self.get_sad_and_attachments(&said)
    }

    fn get_latest_establishment_event(&self, pre: &str) -> Result<(String, u128)> {
        let sn = self.get_kel(pre)?.len() as u32;
        self.get_latest_establishment_event_as_of_sn(pre, sn)
    }

    fn get_latest_establishment_event_as_of_sn(
        &self,
        pre: &str,
        sn: u32,
    ) -> Result<(String, u128)> {
        let mut kel = self.get_kel(pre)?;
        kel.reverse();

        let mut found = false;
        let mut found_sn = 0;
        let mut event = "".to_string();

        for (i, e) in kel.iter().enumerate() {
            found_sn = (kel.len() - i - 1) as u128;

            if found_sn > sn as u128 {
                continue;
            }

            let serder = cesride::Serder::new_with_raw(e.as_bytes())?;
            if serder.est()? {
                found = true;
                event = e.clone();
                break;
            }
        }

        if !found {
            return Err(Error::Value.into());
        }

        Ok((event, found_sn))
    }

    fn get_latest_transaction_event(&self, pre: &str) -> Result<String> {
        if !self.tels.contains_key(pre) {
            return Err(Error::Value.into());
        }

        let saids = self.tels.get(pre).unwrap();
        if saids.is_empty() {
            return Err(Error::Value.into());
        }

        self.get_sad_and_attachments(&saids[saids.len() - 1])
    }

    fn get_latest_key_event_said(&self, pre: &str) -> Result<String> {
        if !self.kels.contains_key(pre) {
            return Err(Error::Value.into());
        }

        let saids = self.kels.get(pre).unwrap();
        if saids.is_empty() {
            return Err(Error::Value.into());
        }

        Ok(saids[saids.len() - 1].clone())
    }

    fn get_latest_establishment_event_said(&self, pre: &str) -> Result<(String, u128)> {
        let (event, found_sn) = self.get_latest_establishment_event(pre)?;
        let serder = cesride::Serder::new_with_raw(event.as_bytes())?;
        Ok((serder.said()?, found_sn))
    }
    fn get_latest_establishment_event_said_as_of_sn(
        &self,
        pre: &str,
        sn: u32,
    ) -> Result<(String, u128)> {
        let (event, found_sn) = self.get_latest_establishment_event_as_of_sn(pre, sn)?;
        let serder = cesride::Serder::new_with_raw(event.as_bytes())?;
        Ok((serder.said()?, found_sn))
    }

    fn get_kel(&self, pre: &str) -> Result<Vec<String>> {
        if !self.kels.contains_key(pre) {
            return Err(Error::Value.into());
        }

        let saids = self.kels.get(pre).unwrap();
        let mut result = vec![];

        for said in saids {
            result.push(self.get_sad_and_attachments(&said)?)
        }

        Ok(result)
    }

    fn get_tel(&self, pre: &str) -> Result<Vec<String>> {
        if !self.tels.contains_key(pre) {
            return Err(Error::Value.into());
        }

        let saids = self.tels.get(pre).unwrap();
        let mut result = vec![];

        for said in saids {
            result.push(self.get_sad_and_attachments(&said)?)
        }

        Ok(result)
    }

    fn count_key_events(&self, pre: &str) -> Result<usize> {
        if !self.kels.contains_key(pre) {
            Ok(0usize)
        } else {
            Ok(self.kels[pre].len())
        }
    }

    fn count_transaction_events(&self, pre: &str) -> Result<usize> {
        if !self.tels.contains_key(pre) {
            Ok(0usize)
        } else {
            Ok(self.tels[pre].len())
        }
    }

    fn count_establishment_events(&self, pre: &str) -> Result<usize> {
        let mut count = 0usize;
        let kel = self.get_kel(pre)?;

        for event in &kel {
            if cesride::Serder::new_with_raw(event.as_bytes())?.est()? {
                count += 1;
            }
        }

        Ok(count)
    }
}

struct Vault {
    store: Store,
    prefix: String,
    registry: String,
}

impl Vault {
    pub fn new() -> Result<(Self, String)> {
        let (aid, keys, icp) = keri::kmi::incept(
            Some(cesride::matter::Codex::CRYSTALS_Dilithium3_Seed),
            None,
            None,
            Some(cesride::matter::Codex::CRYSTALS_Dilithium3_Seed),
            None,
            None,
            None,
            Some(true),
            Some(cesride::matter::Codex::Blake3_256),
            None,
        )?;
        let (registry, vcp) = acdc::tel::management::incept(&aid)?;
        let seal = dat!([{"i": &registry, "s": "0", "d": &registry}]);
        let (ixn_said, ixn) = keri::kmi::interact(&keys[0], &aid, &aid, 1, &seal)?;

        let mut store = Store::new(&aid);
        store.insert_keys(&aid, &keys[0])?;
        store.insert_keys(&aid, &keys[1])?;

        drop(keys);

        let counter = cesride::Counter::new_with_code_and_count(
            cesride::counter::Codex::SealSourceCouples,
            1,
        )?;
        let seqner = cesride::Seqner::new_with_sn(1)?;
        let vcp = vcp + &counter.qb64()? + &seqner.qb64()? + &ixn_said;

        keri::parsing::ingest_messages(
            &mut store,
            &(icp + &ixn + &vcp),
            Some(false),
            Some(true),
            false,
        )?;

        Ok((
            Vault {
                store,
                prefix: aid.clone(),
                registry,
            },
            aid,
        ))
    }

    pub fn issue_acdc(
        &mut self,
        schema: &str,
        data: &str,
        recipient: Option<&str>,
        private: Option<bool>,
        source: Option<&str>,
        rules: Option<&str>,
        partially_disclosable: Option<&str>,
    ) -> Result<String> {
        let (acdc_said, ixn, iss, acdc, sads) = acdc::issue_acdc(
            &self.store,
            &self.registry,
            &self.prefix,
            schema,
            data,
            recipient,
            private,
            source,
            rules,
            partially_disclosable,
        )?;

        keri::parsing::ingest_messages(
            &mut self.store,
            &(ixn + &iss + &acdc),
            Some(false),
            Some(true),
            true,
        )?;
        for sad in &sads {
            self.store.insert_sad(&sad.to_json()?)?;
        }

        Ok(acdc_said)
    }

    pub fn fetch_acdc(&self, said: &str, to_disclose: &[&str], full: bool) -> Result<String> {
        let acdc_string = self.store.get_acdc(said)?;
        let creder = cesride::Creder::new_with_raw(acdc_string.as_bytes())?;

        // in this example we only care about the attributes section
        // it's important to expand the source/edges block to allow verification of edges if they exist
        let mut to_expand = vec![vec!["a"]];
        for key in to_disclose {
            to_expand.push(vec!["a", *key]);
        }

        let expanded_acdc = acdc::expand_acdc(&creder, to_expand.as_slice(), &self.store)?;

        let mut messages = if full {
            let kel = self.store.get_kel(&expanded_acdc.issuer()?)?;
            let mgmt_tel = self.store.get_tel(&creder.status()?.unwrap())?;
            let vc_tel = self.store.get_tel(&creder.said()?)?;

            kel.join("") + &mgmt_tel.join("") + &vc_tel.join("")
        } else {
            "".to_string()
        };

        messages += &(expanded_acdc.crd().to_json()? + &acdc_string[creder.raw().len()..]);
        Ok(messages)
    }

    pub fn ingest_messages(&mut self, messages: &str) -> Result<()> {
        keri::parsing::ingest_messages(&mut self.store, messages, Some(false), Some(true), false)?;
        Ok(())
    }
}

fn main() {
    // TODO: had to break this into pieces to avoid recursion limits in macro parsing
    let schema_attributes_value = dat!({
        "oneOf": [
            {
                "description": "Attributes section SAID",
                "type": "string"
            },
            {
            "$id": "",
            "description": "Attributes section",
            "type": "object",
            "required": [
                "d",
                "dt",
                "i",
                "u",
                "legalName",
                "age"
            ],
            "properties": {
                "d": {
                    "description": "Attributes SAID",
                    "type": "string"
                },
                "dt": {
                    "description": "Date and time of issuance in ISO8601 format",
                    "type": "string",
                    "format": "date-time"
                },
                "i": {
                    "description": "Issuee AID",
                    "type": "string"
                },
                "u": {
                    "description": "Salty Nonce",
                    "type": "string"
                },
                "legalName": {
                    "oneOf": [
                        {
                            "description": "Blinded legal name SAID",
                            "type": "string"
                        },
                        {
                            "type": "object",
                            "required": ["d", "u", "value"],
                            "properties": {
                                "d": {
                                    "description": "SAID of disclosable data",
                                    "type": "string"
                                },
                                "u": {
                                    "description": "Salty nonce",
                                    "type": "string"
                                },
                                "value": {
                                    "description": "Unblinded legal name",
                                    "type": "string"
                                }
                            },
                            "additionalProperties": false
                        }
                    ]
                },
                "age": {
                    "oneOf": [
                        {
                            "description": "Blinded age SAID",
                            "type": "string"
                        },
                        {
                            "type": "object",
                            "required": ["d", "u", "value"],
                            "properties": {
                                "d": {
                                    "description": "SAID of disclosable data",
                                    "type": "string"
                                },
                                "u": {
                                    "description": "Salty nonce",
                                    "type": "string"
                                },
                                "value": {
                                    "description": "Unblinded age",
                                    "type": "number"
                                }
                            },
                            "additionalProperties": false
                        }
                    ]
                }
            },
            "additionalProperties": false
            }
        ]
    });

    let mut schema_value = dat!({
        "$id": "",
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Nested Partial Disclosure",
        "description": "A demonstration of Nested Partial Disclosure",
        "credentialType": "Demonstration",
        "version": "1.0.0",
        "type": "object",
        "required": [
          "v",
          "d",
          "i",
          "s",
          "ri",
          "a"
        ],
        "properties": {
          "v": {
            "description": "Credential Version",
            "type": "string"
          },
          "d": {
            "description": "Credential SAID",
            "type": "string"
          },
          "u": {
            "description": "One time use nonce - optional",
            "type": "string"
          },
          "i": {
            "description": "Issuer AID",
            "type": "string"
          },
          "ri": {
            "description": "Credential Registry Identifier",
            "type": "string"
          },
          "s": {
            "description": "Schema SAID",
            "type": "string"
          },
          "a": schema_attributes_value
        },
        "additionalProperties": false
    });

    let (saidified_value, _) =
        keri::saidify_value(&mut schema_value, Some("$id"), Some(false), Some(false)).unwrap();
    let schema_string = saidified_value.to_json().unwrap();
    println!("constructed schema:");
    println!("{schema_string}");
    println!();

    let schema = saidified_value["$id"].to_string().unwrap();

    let schemer =
        acdc::schemer::Schemer::new(Some(schema_string.as_bytes()), None, None, None).unwrap();
    acdc::schemer::cache().prime(&[schemer]).unwrap();
    println!("primed schema cache");
    println!();

    let (mut issuer_vault, _issuer_aid) = Vault::new().unwrap();
    println!("incepted `issuer` vault");
    let (mut issuee_vault, issuee_aid) = Vault::new().unwrap();
    println!("incepted `issuee` vault");
    let (mut disclosee_vault, _disclosee_aid) = Vault::new().unwrap();
    println!("incepted `disclosee` vault");
    println!();

    let said = issuer_vault
        .issue_acdc(
            &schema,
            "{}",
            Some(&issuee_aid),
            Some(true),
            None,
            None,
            Some(
                &dat!({"legalName": "Jason Colburne", "age": 43})
                    .to_json()
                    .unwrap(),
            ),
        )
        .unwrap();
    println!("issued acdc from `issuer` vault");

    let issuance = issuer_vault
        .fetch_acdc(&said, &["legalName", "age"], true)
        .unwrap();
    println!("fetched complete acdc from `issuer` vault:");
    println!("{issuance}");
    println!();

    issuee_vault.ingest_messages(&issuance).unwrap();
    println!("ingested acdc into `issuee` vault");
    println!();

    let presentation = issuee_vault
        .fetch_acdc(&said, &["legalName"], true)
        .unwrap();
    println!("fetched partial acdc for disclosure from `issuee` vault:");
    println!("{presentation}");
    println!();

    disclosee_vault.ingest_messages(&presentation).unwrap();
    println!("ingested partial acdc into `disclosee` vault");
}
