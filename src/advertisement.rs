use crate::interfaces::*;
use crate::introspect::*;
use crate::*;

use rustbus::params;
use rustbus::params::{Base, Param};
use rustbus::signature;
use std::time::Instant;

pub struct Advertisement {
    pub typ: AdType,
    pub service_uuids: Vec<UUID>,
    pub manu_data: HashMap<u16, ([u8; 27], usize)>,
    pub serv_dict: HashMap<UUID, ([u8; 27], usize)>,
    pub solicit_uuids: Vec<UUID>,
    pub includes: Vec<String>,
    pub timeout: u16,
    pub duration: u16,
    pub(crate) index: u16,
    pub(crate) path: PathBuf,
    pub appearance: u16,
    pub localname: String,
    timeout_start: Instant,
    duration_start: Instant,
}
impl Advertisement {
    pub fn new(typ: AdType, localname: String) -> Self {
        let now = Instant::now();
        Advertisement {
            typ,
            service_uuids: Vec::new(),
            solicit_uuids: Vec::new(),
            includes: Vec::new(),
            timeout: 180,
            timeout_start: now,
            duration: 180,
            duration_start: now,
            appearance: 0,
            localname,
            index: 0,
            path: PathBuf::new(),
            manu_data: HashMap::new(),
            serv_dict: HashMap::new(),
        }
    }
    fn timeout(&self) -> u16 {
        let secs = Instant::now().duration_since(self.timeout_start).as_secs();
        (self.timeout as u64).saturating_sub(secs) as u16
    }
    fn duration(&self) -> u16 {
        let secs = Instant::now().duration_since(self.timeout_start).as_secs();
        (self.duration as u64).saturating_sub(secs) as u16
    }
    pub fn validate(&self) -> Result<(), Error> {
        for uuid in &self.service_uuids {
            if !validate_uuid(uuid) {
                return Err(Error::BadInput(format!(
                    "{} is an invalid uuid in service_uuids",
                    uuid
                )));
            }
        }
        for uuid in &self.solicit_uuids {
            if !validate_uuid(uuid) {
                return Err(Error::BadInput(format!(
                    "{} is an invalid uuid in solicit_uuids",
                    uuid
                )));
            }
        }

        Ok(())
    }
}
pub enum AdType {
    Peripheral,
    Broadcast,
}
impl ToString for AdType {
    fn to_string(&self) -> String {
        match self {
            AdType::Broadcast => "broadcast".to_string(),
            AdType::Peripheral => "peripheral".to_string(),
        }
    }
}
impl AdType {
    pub fn to_str(&self) -> &'static str {
        match self {
            AdType::Peripheral => "peripheral",
            AdType::Broadcast => "broadcast",
        }
    }
}

impl Properties for Advertisement {
    const INTERFACES: &'static [(&'static str, &'static [&'static str])] = &[LEAD_IF, PROP_IF];
    fn get_inner<'a, 'b>(&mut self, interface: &str, prop: &str) -> Option<Param<'a, 'b>> {
        match interface {
            LEAD_IF_STR => match prop {
                TYPE_PROP => Some(base_param_to_variant(self.typ.to_string().into())),
                SERV_UUIDS_PROP => {
                    let service_uuids: Vec<Param> = self
                        .service_uuids
                        .iter()
                        .map(|x| Param::Base(x.to_string().into()))
                        .collect();
                    let array = params::Array {
                        values: service_uuids,
                        element_sig: signature::Type::Base(signature::Base::String),
                    };
                    Some(container_param_to_variant(Container::Array(array)))
                }
                MANU_DATA_PROP => {
                    //let manu_data = Param::Container(Container::Array(params::Array));
                    let base = signature::Type::Base(signature::Base::Byte);
                    let typ = signature::Type::Container(signature::Container::Array(Box::new(
                        base.clone(),
                    )));
                    let manu_data: HashMap<Base, Param> = self
                        .manu_data
                        .iter()
                        .map(|(key, (v, l))| {
                            let key = Base::Uint16(*key);
                            let byte_vec: Vec<Param> = v[..*l]
                                .iter()
                                .map(|x| Param::Base(Base::Byte(*x)))
                                .collect();
                            let array = Param::Container(Container::Array(params::Array {
                                element_sig: base.clone(),
                                values: byte_vec,
                            }));
                            (key, array)
                        })
                        .collect();
                    let manu_data_dict = params::Dict {
                        key_sig: signature::Base::Uint16,
                        value_sig: typ,
                        map: manu_data,
                    };
                    let cont = Container::Dict(manu_data_dict);
                    Some(container_param_to_variant(cont))
                }
                SOLICIT_UUIDS_PROP => {
                    let uuids: Vec<Param> = self
                        .solicit_uuids
                        .iter()
                        .map(|x| Param::Base(Base::String(x.to_string())))
                        .collect();
                    let array = params::Array {
                        values: uuids,
                        element_sig: signature::Type::Base(signature::Base::String),
                    };
                    Some(container_param_to_variant(Container::Array(array)))
                }
                SERV_DATA_PROP => {
                    let base = signature::Type::Base(signature::Base::Byte);
                    let typ = signature::Type::Container(signature::Container::Array(Box::new(
                        base.clone(),
                    )));
                    let serv_data: HashMap<Base, Param> = self
                        .serv_dict
                        .iter()
                        .map(|(key, (v, l))| {
                            let key = Base::String(key.to_string());
                            let byte_vec: Vec<Param> = v[..*l]
                                .iter()
                                .map(|x| Param::Base(Base::Byte(*x)))
                                .collect();
                            let array = Param::Container(Container::Array(params::Array {
                                element_sig: base.clone(),
                                values: byte_vec,
                            }));
                            (key, array)
                        })
                        .collect();
                    let serv_data_dict = params::Dict {
                        key_sig: signature::Base::String,
                        value_sig: typ,
                        map: serv_data,
                    };
                    let cont = Container::Dict(serv_data_dict);
                    Some(container_param_to_variant(cont))
                }
                DATA_PROP => unimplemented!(),
                /*
                DISCOVERABLE_PROP => unimplemented!(),
                DISCOVERABLE_TO_PROP => unimplemented!(),*/
                INCLUDES_PROP => {
                    let includes: Vec<Param> = self
                        .includes
                        .iter()
                        .map(|x| Param::Base(x.to_string().into()))
                        .collect();
                    let array = params::Array {
                        values: includes,
                        element_sig: signature::Type::Base(signature::Base::String),
                    };
                    Some(container_param_to_variant(Container::Array(array)))
                }
                LOCAL_NAME_PROP => Some(base_param_to_variant(self.localname.to_string().into())),
                APPEARANCE_PROP => Some(base_param_to_variant(self.appearance.into())),
                DURATION_PROP => {
                    let duration = self.duration as u64;
                    let time = duration.saturating_sub(
                        Instant::now().duration_since(self.duration_start).as_secs(),
                    );
                    Some(base_param_to_variant((time as u16).into()))
                }
                TO_PROP => {
                    let timeout = self.timeout as u64;
                    let time = timeout.saturating_sub(
                        Instant::now().duration_since(self.timeout_start).as_secs(),
                    );
                    Some(base_param_to_variant((time as u16).into()))
                }
                //TODO:implement SND_CHANNEL_PROP => unimplemented!(),
                _ => None,
            },
            _ => None,
        }
    }
    fn set_inner(&mut self, _interface: &str, _prop: &str, _val: Variant) -> Option<String> {
        unimplemented!()
    }
}

impl Introspectable for Advertisement {
    fn introspectable_str(&self) -> String {
        let mut ret = String::new();
        ret.push_str(INTROSPECT_FMT_P1);
        ret.push_str(self.path.to_str().unwrap());
        ret.push_str(INTROSPECT_FMT_P2);
        ret.push_str(PROP_STR);
        ret.push_str(ADV_STR);
        ret.push_str(INTROSPECT_FMT_P3);
        ret
    }
}
