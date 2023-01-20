use std::{
    collections::{BTreeMap, HashMap},
    fs::OpenOptions,
    io::{BufReader, Cursor},
    path::Path,
};

use crate::{classes::p_ptr::PPtr, type_tree::TypeTreeObject, SerializedFile, UnityFS};

pub struct UnityAssetViewer {
    pub cab_maps: HashMap<String, i64>,
    pub serialized_file_map: BTreeMap<i64, SerializedFile>,
    serialized_file_count: i64,
    unity_fs_map: BTreeMap<i64, UnityFS>,
    unity_fs_count: i64,
    serialized_file_to_unity_fs_map: BTreeMap<i64, i64>,
    pub container_maps: HashMap<String, Vec<(i64, PPtr)>>,
    container_name_maps: HashMap<i64, HashMap<i64, String>>,
}

impl UnityAssetViewer {
    pub fn new() -> Self {
        Self {
            cab_maps: HashMap::new(),
            serialized_file_map: BTreeMap::new(),
            serialized_file_count: 0,
            unity_fs_map: BTreeMap::new(),
            unity_fs_count: 0,
            serialized_file_to_unity_fs_map: BTreeMap::new(),
            container_maps: HashMap::new(),
            container_name_maps: HashMap::new(),
        }
    }

    pub fn read_dir<P: AsRef<Path>>(&mut self, dir_path: P) -> anyhow::Result<()> {
        let dirs = std::fs::read_dir(dir_path)?;
        for entry in dirs {
            if let Ok(entry) = entry {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let file = OpenOptions::new().read(true).open(entry.path())?;
                        let file = BufReader::new(file);

                        let unity_fs = UnityFS::read(Box::new(file), None)?;

                        let unity_fs_id = self.unity_fs_count;
                        self.unity_fs_count = self.unity_fs_count + 1;

                        for cab_path in unity_fs.get_cab_path() {
                            let cab_buff = unity_fs.get_file_by_path(&cab_path)?;

                            let serialized_file_id = self.serialized_file_count;
                            self.serialized_file_count = self.serialized_file_count + 1;

                            let cab_buff_reader = Box::new(Cursor::new(cab_buff));
                            let serialized_file =
                                SerializedFile::read(cab_buff_reader, serialized_file_id)?;

                            if let Ok(Some(asset_bundle)) =
                                serialized_file.get_tt_object_by_path_id(1)
                            {
                                if let Some(containers) = asset_bundle
                                    .get_string_key_map_by_path("/Base/m_Container/Array")
                                {
                                    let mut name_map = HashMap::new();
                                    for (name, asset_info) in containers {
                                        if let Some(pptr) =
                                            asset_info.get_object_by_path("/Base/asset")
                                        {
                                            let pptr = PPtr::new(pptr);
                                            if let Some(path_id) = pptr.get_path_id() {
                                                name_map.insert(path_id, name.clone());
                                            }

                                            if let Some(objs) = self.container_maps.get_mut(&name) {
                                                objs.push((serialized_file_id, pptr));
                                            } else {
                                                self.container_maps
                                                    .insert(name, vec![(serialized_file_id, pptr)]);
                                            }
                                        }
                                    }
                                    self.container_name_maps
                                        .insert(serialized_file_id, name_map);
                                }
                            }

                            self.serialized_file_map
                                .insert(serialized_file_id, serialized_file);
                            self.serialized_file_to_unity_fs_map
                                .insert(serialized_file_id, unity_fs_id);
                            self.cab_maps.insert(cab_path, serialized_file_id);
                        }

                        self.unity_fs_map.insert(unity_fs_id, unity_fs);
                    }
                } else {
                    println!("Couldn't get file type for {:?}", entry.path());
                }
            }
        }
        Ok(())
    }

    pub fn get_serialized_file_by_path(&self, path: &String) -> Option<&SerializedFile> {
        if let Some(serialized_file_id) = self.cab_maps.get(path) {
            if let Some(serialized_file) = self.serialized_file_map.get(serialized_file_id) {
                return Some(serialized_file);
            }
        }
        None
    }

    pub fn get_unity_fs_by_cab_path(&self, path: &String) -> Option<&UnityFS> {
        if let Some(serialized_file_id) = self.cab_maps.get(path) {
            if let Some(unity_fs_id) = self.serialized_file_to_unity_fs_map.get(serialized_file_id)
            {
                if let Some(unity_fs) = self.unity_fs_map.get(unity_fs_id) {
                    return Some(unity_fs);
                }
            }
        }
        None
    }

    pub fn get_unity_fs_by_pptr(&self, pptr: &PPtr) -> Option<&UnityFS> {
        let serialized_file_id = pptr.get_serialized_file_id();
        if let Some(unity_fs_id) = self
            .serialized_file_to_unity_fs_map
            .get(&serialized_file_id)
        {
            if let Some(unity_fs) = self.unity_fs_map.get(unity_fs_id) {
                return Some(unity_fs);
            }
        }

        None
    }

    pub fn get_unity_fs_by_type_tree_object(
        &self,
        type_tree_object: &TypeTreeObject,
    ) -> Option<&UnityFS> {
        if let Some(unity_fs_id) = self
            .serialized_file_to_unity_fs_map
            .get(&type_tree_object.serialized_file_id)
        {
            if let Some(unity_fs) = self.unity_fs_map.get(unity_fs_id) {
                return Some(unity_fs);
            }
        }
        None
    }

    pub fn get_container_name_by_path_id(
        &self,
        cab_name: &String,
        path_id: i64,
    ) -> Option<&String> {
        if let Some(serialized_file_id) = self.cab_maps.get(cab_name) {
            if let Some(name_map) = self.container_name_maps.get(serialized_file_id) {
                return name_map.get(&path_id);
            }
        }
        None
    }

    pub fn get_container_name_by_pptr(&self, pptr: &PPtr) -> Option<&String> {
        let serialized_file_id = pptr.get_serialized_file_id();
        if let Some(name_map) = self.container_name_maps.get(&serialized_file_id) {
            if let Some(path_id) = pptr.get_path_id() {
                return name_map.get(&path_id);
            }
        }
        None
    }

    pub fn get_type_tree_object_by_container_name(
        &self,
        container_name: &String,
    ) -> anyhow::Result<Option<TypeTreeObject>> {
        if let Some(serialized_file_id) = self.container_maps.get(container_name) {
            if let Some((serialized_file_id, pptr)) = serialized_file_id.get(0) {
                if let Some(serialized_file) = self.serialized_file_map.get(serialized_file_id) {
                    return pptr.get_type_tree_object(serialized_file, Some(self));
                }
            }
        }
        Ok(None)
    }

    pub fn get_serialized_file_by_container_name(
        &self,
        container_name: &String,
    ) -> Option<&SerializedFile> {
        if let Some(serialized_file_id) = self.container_maps.get(container_name) {
            if let Some((serialized_file_id, _pptr)) = serialized_file_id.get(0) {
                return self.serialized_file_map.get(serialized_file_id);
            }
        }
        None
    }
}