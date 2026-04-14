use std::{collections::HashMap, fmt::Debug, str::FromStr};

use chrono::NaiveDateTime;
use log::{debug, error, trace, warn};

use crate::error::SFError;











#[ouroboros::self_referencing]
pub struct Response {
    body: String,
    #[borrows(body)]
    #[covariant]
    resp: HashMap<&'this str, ResponseVal<'this>>,
    
    
    
    received_at: NaiveDateTime,
}

impl Clone for Response {
    
    #[allow(clippy::expect_used)]
    fn clone(&self) -> Self {
        Self::parse(self.raw_response().to_string(), self.received_at())
            .expect("Invalid response cloned")
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.values().iter().map(|a| (a.0, a.1.as_str())))
            .finish()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Response", 2)?;
        s.serialize_field("body", self.borrow_body())?;
        s.serialize_field("received_at", &self.received_at())?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AVisitor;

        impl<'de> serde::de::Visitor<'de> for AVisitor {
            type Value = Response;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str(
                    "struct Response with fields body and received_at",
                )
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut body = None;
                let mut received_at = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "body" => {
                            body = Some(map.next_value()?);
                        }
                        "received_at" => {
                            received_at = Some(map.next_value()?);
                        }
                        _ => {
                            
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                let body: String =
                    body.ok_or_else(|| serde::de::Error::missing_field("q"))?;
                let received_at: NaiveDateTime = received_at
                    .ok_or_else(|| serde::de::Error::missing_field("j"))?;

                Response::parse(body, received_at).map_err(|_| {
                    serde::de::Error::custom("invalid response body")
                })
            }
        }

        deserializer.deserialize_struct(
            "Response",
            &["body", "received_at"],
            AVisitor,
        )
    }
}

impl Response {
    
    
    #[must_use]
    pub fn values(&self) -> &HashMap<&str, ResponseVal<'_>> {
        self.borrow_resp()
    }

    
    
    
    
    #[must_use]
    pub fn raw_response(&self) -> &str {
        self.borrow_body()
    }

    
    #[must_use]
    pub fn received_at(&self) -> NaiveDateTime {
        self.with_received_at(|a| *a)
    }

    
    
    
    
    
    
    
    
    
    
    pub fn parse(
        og_body: String,
        received_at: NaiveDateTime,
    ) -> Result<Response, SFError> {
        
        

        
        
        

        let body = og_body
            .trim_end_matches('|')
            .trim_start_matches(|a: char| !a.is_alphabetic());
        trace!("Received raw response: {body}");

        if !body.contains(':')
            && !body.starts_with("success")
            && !body.starts_with("Success")
        {
            return Err(SFError::ParsingError(
                "unexpected server response",
                body.to_string(),
            ));
        }

        if body.starts_with("error") || body.starts_with("Error") {
            let raw_error = body.split_once(':').unwrap_or_default().1;

            let error_msg = match raw_error {
                "adventure index must be 1-3" => "quest index must be 0-2",
                x => x,
            };

            return Err(SFError::ServerError(error_msg.to_string()));
        }

        let resp = ResponseBuilder {
            body: og_body,
            resp_builder: |body: &String| {
                let mut res = HashMap::new();
                for part in body
                    .trim_start_matches(|a: char| !a.is_alphabetic())
                    .trim_end_matches('|')
                    .split('&')
                    .filter(|a| !a.is_empty())
                {
                    let Some((full_key, value)) = part.split_once(':') else {
                        warn!("weird k/v in resp: {part}");
                        continue;
                    };

                    let (key, sub_key) = match full_key.split_once('.') {
                        Some(x) => {
                            
                            x
                        }
                        None => {
                            if let Some((k, sk)) = full_key.split_once('(') {
                                
                                (k, sk.trim_matches(')'))
                            } else {
                                
                                (full_key, "")
                            }
                        }
                    };
                    if key.is_empty() {
                        continue;
                    }

                    let old_val =
                        res.insert(key, ResponseVal { value, sub_key });
                    if let Some(old_val) = old_val {
                        let old = old_val.as_str();
                        debug!("Overwrote [{key}]: {old} => {value}");
                    }
                }
                res
            },
            received_at,
        }
        .build();

        Ok(resp)
    }
}




#[derive(Debug, Clone, Copy)]
#[allow(clippy::module_name_repetitions)]
pub struct ResponseVal<'a> {
    value: &'a str,
    sub_key: &'a str,
}

impl ResponseVal<'_> {
    
    
    
    
    
    pub fn into<T: FromStr>(self, name: &'static str) -> Result<T, SFError> {
        self.value.trim().parse().map_err(|_| {
            error!("Could not convert {name} into target type: {self}");
            SFError::ParsingError(name, self.value.to_string())
        })
    }

    
    
    
    
    
    
    
    pub fn into_list<T: FromStr>(
        self,
        name: &'static str,
    ) -> Result<Vec<T>, SFError> {
        let x = &self.value;
        if x.is_empty() {
            return Ok(Vec::new());
        }
        
        x.trim_matches(|a| ['/', ' ', '\n'].contains(&a))
            .split('/')
            .map(|c| {
                c.trim().parse::<T>().map_err(|_| {
                    error!(
                        "Could not convert {name} into list because of {c}: \
                         {self}"
                    );
                    SFError::ParsingError(name, format!("{c:?}"))
                })
            })
            .collect()
    }

    
    
    
    
    
    
    #[must_use]
    pub fn sub_key(&self) -> &str {
        self.sub_key
    }

    
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value
    }
}

impl std::fmt::Display for ResponseVal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.value)
    }
}
