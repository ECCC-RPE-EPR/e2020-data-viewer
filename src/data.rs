use std::{ops::Range, path::PathBuf};

use color_eyre::eyre::Result;
use hdf5::{
    types::{FixedUnicode, VarLenUnicode},
    Dataset, Selection,
};
use ndarray::{Array2, ArrayD};

#[derive(Debug, Clone)]
pub struct Data {
    pub name: String,
    pub doc: String,
    pub units: String,
    pub set_names: Vec<String>,
    pub ndims: usize,
    pub typ: String,
    pub shape: Vec<usize>,
    pub dataset: Dataset,
    pub set_data: Vec<Vec<String>>,
}

impl Data {
    pub fn new(file: PathBuf, name: String) -> Result<Self> {
        let f = hdf5::File::open(file)?;
        let dataset = f.dataset(&name)?;
        let name = dataset.name();
        let units = dataset
            .attr("units")?
            .as_reader()
            .read_scalar::<FixedUnicode<100>>()?
            .to_string();
        let doc = dataset
            .attr("doc")?
            .as_reader()
            .read_scalar::<FixedUnicode<100>>()?
            .to_string();
        let typ = dataset
            .attr("type")?
            .as_reader()
            .read_scalar::<FixedUnicode<100>>()?
            .to_string();
        let ndims = dataset.shape().len();
        let set_names = dataset
            .attr("dims")?
            .read_1d::<VarLenUnicode>()?
            .into_iter()
            .map(|dim| dim.to_string())
            .collect::<Vec<_>>();
        let mut shape = dataset.shape();
        shape.reverse();
        let mut set_data = vec![];
        let g_name = name
            .split('/')
            .filter(|s| !(s.is_empty()))
            .collect::<Vec<&str>>()[0];
        for dim in set_names.iter() {
            let ds = f.dataset(format!("{}/{}", g_name, dim).as_str())?;
            let set = ds
                .read_1d::<VarLenUnicode>()?
                .into_iter()
                .map(|dim| dim.to_string())
                .collect::<Vec<_>>();
            set_data.push(set);
        }
        Ok(Self {
            name,
            units,
            doc,
            typ,
            set_names,
            ndims,
            shape,
            dataset,
            set_data,
        })
    }

    pub fn selection(&self, range_x: Range<usize>, range_y: Range<usize>) -> Selection {
        let mut points = Vec::new();

        for x in range_x {
            for y in range_y.clone() {
                points.push([x, y]);
            }
        }

        Selection::Points(
            Array2::from_shape_vec((points.len(), 2), points.into_iter().flatten().collect())
                .unwrap(),
        )
    }
}

mod tests {
    use color_eyre::eyre::Result;

    use super::*;

    #[test]
    fn test_dataset() -> Result<()> {
        let file = "./.data/database.hdf5".into();
        let name = "iinput/FsPEE".to_string();
        Data::new(file, name)?;
        Ok(())
    }
}
