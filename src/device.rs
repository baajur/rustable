use crate::gatt::*;
use crate::{Bluetooth, Error, ToMAC, ToUUID, Variant, MAC, UUID};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub enum AddrType {
    Public,
    Random,
}
pub trait Device<'a>: HasChildren<'a> {
    type ServiceType: Service<'a>;
    fn services(&mut self) -> Vec<UUID>;
    fn get_service(&'a mut self, uuid: &UUID) -> Option<Self::ServiceType>;
    fn has_service(&self, uuid: &UUID) -> bool;
    fn address(&self) -> &MAC;
    fn address_type(&mut self) -> AddrType;
    fn name(&mut self) -> String;
}
pub(crate) struct RemoteDeviceBase {
    pub(crate) mac: MAC,
    pub(crate) path: PathBuf,
    pub(crate) services: HashMap<MAC, RemoteServiceBase>,
    connected: Rc<Cell<bool>>,
    paired: Rc<Cell<bool>>,
    //comp_map: HashMap<OsString, MAC>,
}
impl RemoteDeviceBase {
    pub(crate) fn from_props(
        mut value: HashMap<String, Variant>,
        path: PathBuf,
    ) -> Result<Self, Error> {
        let mac = match value.remove("Address") {
            Some(addr) => match addr.get::<String>() {
                Ok(mac) => mac.to_mac(),
                Err(_) => {
                    return Err(Error::DbusReqErr(
                        "Invalid device returned; Address field is invalid type".to_string(),
                    ))
                }
            },
            None => {
                return Err(Error::DbusReqErr(
                    "Invalid device returned; missing Address field".to_string(),
                ))
            }
        };
        let connected = match value.remove("Connected") {
            Some(con) => match con.get::<bool>() {
                Ok(con) => Rc::new(Cell::new(con)),
                Err(_) => {
                    return Err(Error::DbusReqErr(
                        "Invalid device returned; Connected field is invalid type".to_string(),
                    ))
                }
            },
            None => {
                return Err(Error::DbusReqErr(
                    "Invalid device returned; missing Connected field".to_string(),
                ))
            }
        };
        let paired = match value.remove("Paired") {
            Some(paired) => match paired.get::<bool>() {
                Ok(paired) => Rc::new(Cell::new(paired)),
                Err(_) => {
                    return Err(Error::DbusReqErr(
                        "Invalid device returned; Paired field is invalid type".to_string(),
                    ))
                }
            },
            None => {
                return Err(Error::DbusReqErr(
                    "Invalid device returned; missing Paired field".to_string(),
                ))
            }
        };
        Ok(RemoteDeviceBase {
            mac,
            path,
            connected,
            paired,
            services: HashMap::new(),
        })
    }
    pub(crate) fn update_from_changed(
        &mut self,
        changed: HashMap<String, Variant>,
    ) -> Result<(), Error> {
        for (prop, var) in changed {
            match prop.as_str() {
                "Connected" => self.connected.set(var.get()?),
                "Paired" => self.paired.set(var.get()?),
                _ => (),
            }
        }
        Ok(())
    }
    pub(crate) fn match_dev(
        &mut self,
        path: &Path,
    ) -> Option<Option<(UUID, Option<(UUID, Option<UUID>)>)>> {
        match path.strip_prefix(self.path().file_name().unwrap()) {
            Ok(remaining) => {
                if remaining == Path::new("") {
                    Some(None)
                } else {
                    let r_str = remaining.to_str().unwrap();
                    if &r_str[..4] != "serv" {
                        return None;
                    }
                    for uuid in self.get_children() {
                        if let Some(matc) =
                            match_remote_serv(&mut self.get_child(&uuid).unwrap(), remaining)
                        {
                            return Some(Some((uuid, matc)));
                        }
                    }
                    None
                }
            }
            Err(_) => None,
        }
    }
}
impl AttObject for RemoteDeviceBase {
    fn path(&self) -> &Path {
        &self.path
    }
    fn uuid(&self) -> &UUID {
        &self.mac
    }
}
impl<'a> HasChildren<'a> for RemoteDeviceBase {
    type Child = &'a mut RemoteServiceBase;
    fn get_children(&self) -> Vec<UUID> {
        self.services.keys().map(|x| x.clone()).collect()
    }
    fn get_child<T: ToUUID>(&'a mut self, uuid: T) -> Option<Self::Child> {
        let uuid = uuid.to_uuid();
        self.services.get_mut(&uuid)
    }
}

pub struct RemoteDevice<'a> {
    pub(crate) mac: MAC,
    pub(crate) blue: &'a mut Bluetooth,
    #[cfg(feature = "unsafe-opt")]
    ptr: *mut RemoteDeviceBase,
}
impl RemoteDevice<'_> {
    fn get_base(&self) -> &RemoteDeviceBase {
        #[cfg(feature = "unsafe-opt")]
        {
            return &*self.ptr;
        }
        &self.blue.devices[&self.mac]
    }
    fn get_base_mut(&mut self) -> &mut RemoteDeviceBase {
        #[cfg(feature = "unsafe-opt")]
        {
            return &mut *self.ptr;
        }
        self.blue.devices.get_mut(&self.mac).unwrap()
    }
    #[inline]
    pub fn connected(&self) -> bool {
        self.get_base().connected.get()
    }
    pub fn connect(&mut self) -> Result<(), Error> {
        unimplemented!()
    }
    #[inline]
    pub fn paired(&self) -> bool {
        self.get_base().paired.get()
    }
    pub fn pair(&mut self) -> Result<(), Error> {
        unimplemented!()
    }
    pub fn forget_service(&mut self, uuid: &UUID) -> bool {
        self.get_base_mut().services.remove(uuid).is_some()
    }
}

impl<'a, 'b: 'a> HasChildren<'a> for RemoteDevice<'b> {
    type Child = RemoteService<'a, 'b>;
    fn get_children(&self) -> Vec<UUID> {
        self.get_base().get_children()
    }
    fn get_child<T: ToUUID>(&'a mut self, uuid: T) -> Option<Self::Child> {
        unimplemented!()
    }
}
impl<'a, 'c: 'a> Device<'a> for RemoteDevice<'c> {
    type ServiceType = RemoteService<'a, 'c>;
    fn services(&mut self) -> Vec<UUID> {
        self.get_base_mut()
            .services
            .keys()
            .map(|x| x.clone())
            .collect()
    }
    fn get_service(&'a mut self, uuid: &UUID) -> Option<Self::ServiceType> {
        let base = self.get_base_mut();
        let _serv_base = base.services.get_mut(uuid)?;
        Some(RemoteService {
            dev: self,
            uuid: uuid.clone(),
            #[cfg(feature = "unsafe-opt")]
            base: _serv_base,
        })
    }
    fn has_service(&self, uuid: &UUID) -> bool {
        self.get_base().services.contains_key(uuid)
    }
    fn address(&self) -> &MAC {
        &self.mac
    }
    fn address_type(&mut self) -> AddrType {
        unimplemented!()
    }
    fn name(&mut self) -> String {
        unimplemented!()
    }
}
impl<'a> HasChildren<'a> for Bluetooth {
    type Child = &'a mut LocalServiceBase;
    fn get_children(&self) -> Vec<UUID> {
        self.services.keys().map(|x| x.clone()).collect()
    }
    fn get_child<T: ToUUID>(&'a mut self, uuid: T) -> Option<Self::Child> {
        let uuid = uuid.to_uuid();
        self.services.get_mut(&uuid)
    }
}
impl<'a, 'b: 'a, 'c: 'a> Device<'a> for Bluetooth {
    type ServiceType = LocalService<'a>;
    fn services(&mut self) -> Vec<UUID> {
        self.services.keys().map(|x| x.clone()).collect()
    }
    fn get_service(&'a mut self, uuid: &UUID) -> Option<Self::ServiceType> {
        let _base = self.services.get_mut(uuid)?;
        Some(LocalService {
            bt: self,
            uuid: uuid.clone(),
            #[cfg(feature = "unsafe-opt")]
            ptr: _base,
        })
    }
    fn has_service(&self, uuid: &UUID) -> bool {
        self.devices.contains_key(uuid)
    }
    fn address(&self) -> &MAC {
        unimplemented!()
    }
    fn address_type(&mut self) -> AddrType {
        unimplemented!()
    }
    fn name(&mut self) -> String {
        unimplemented!()
    }
}
