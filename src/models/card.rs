use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
#[cfg(feature = "postgres")]
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// A Magic: The Gathering card from the Scryfall database
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "postgres", derive(FromRow))]
pub struct Card {
    pub id: Uuid,
    pub oracle_id: Option<Uuid>,
    pub name: String,
    pub mana_cost: Option<String>,
    pub cmc: Option<f64>,
    pub type_line: Option<String>,
    pub oracle_text: Option<String>,
    pub colors: Option<Vec<String>>,
    pub color_identity: Option<Vec<String>>,
    pub set_code: Option<String>,
    pub set_name: Option<String>,
    pub collector_number: Option<String>,
    pub rarity: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub loyalty: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub prices: Option<serde_json::Value>,
    pub image_uris: Option<serde_json::Value>,
    pub card_faces: Option<serde_json::Value>,
    pub legalities: Option<serde_json::Value>,
    pub released_at: Option<NaiveDate>,
    pub raw_json: serde_json::Value,
    #[serde(skip_deserializing)]
    pub created_at: Option<chrono::NaiveDateTime>,
    #[serde(skip_deserializing)]
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl Card {
    /// Create a Card from raw Scryfall JSON
    pub fn from_scryfall_json(value: serde_json::Value) -> Result<Self> {
        // Extract fields from the JSON value
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .context("Missing or invalid 'id' field")?;

        let oracle_id = value
            .get("oracle_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .context("Missing 'name' field")?
            .to_string();

        let mana_cost = value
            .get("mana_cost")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let cmc = value.get("cmc").and_then(|v| v.as_f64());

        let type_line = value
            .get("type_line")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let oracle_text = value
            .get("oracle_text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let colors = value.get("colors").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        });

        let color_identity = value
            .get("color_identity")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });

        let set_code = value
            .get("set")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let set_name = value
            .get("set_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let collector_number = value
            .get("collector_number")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let rarity = value
            .get("rarity")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let power = value
            .get("power")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let toughness = value
            .get("toughness")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let loyalty = value
            .get("loyalty")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let keywords = value.get("keywords").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        });

        let prices = value.get("prices").cloned();
        let image_uris = value.get("image_uris").cloned();
        let card_faces = value.get("card_faces").cloned();
        let legalities = value.get("legalities").cloned();

        let released_at = value
            .get("released_at")
            .and_then(|v| v.as_str())
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        Ok(Card {
            id,
            oracle_id,
            name,
            mana_cost,
            cmc,
            type_line,
            oracle_text,
            colors,
            color_identity,
            set_code,
            set_name,
            collector_number,
            rarity,
            power,
            toughness,
            loyalty,
            keywords,
            prices,
            image_uris,
            card_faces,
            legalities,
            released_at,
            raw_json: value,
            created_at: None,
            updated_at: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_from_scryfall_json() {
        let json = serde_json::json!({
            "id": "550c74d4-1fcb-406a-b02a-639a760a4380",
            "oracle_id": "39ce6789-1c18-4d61-bbb9-e6c1e6e1e1c1",
            "name": "Lightning Bolt",
            "mana_cost": "{R}",
            "cmc": 1.0,
            "type_line": "Instant",
            "oracle_text": "Lightning Bolt deals 3 damage to any target.",
            "colors": ["R"],
            "color_identity": ["R"],
            "set": "lea",
            "set_name": "Limited Edition Alpha",
            "collector_number": "161",
            "rarity": "common",
            "keywords": [],
            "released_at": "1993-08-05"
        });

        let card = Card::from_scryfall_json(json).unwrap();
        assert_eq!(card.name, "Lightning Bolt");
        assert_eq!(card.mana_cost, Some("{R}".to_string()));
        assert_eq!(card.cmc, Some(1.0));
    }
}
