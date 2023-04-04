import argparse
import os
import h5py as hdf
import logging
from pathlib import Path
from typing import List
import matplotlib.pyplot as plt

class Timeseries:
    def __init__(self, timeseries, str_id, unit, scale):
        self.str_id = str_id
        self.timeseries = timeseries
        self.unit = unit
        self.scale = scale

def os_file(path) -> bool:
    is_file = os.path.isfile(path)
    # Get suffix and ditch the "."
    is_hdf5 = Path(path).suffix.lower()[1:] in ["hdf5", "h5"]
    if is_file and is_hdf5:
        return path

    raise argparse.ArgumentTypeError(f"{path} is not an HDF5 file!")


def get_data_for(logfile, canids) -> List[Timeseries]:
    def to_timeseries(key, file):
        item = file.get(key)
        str_id = key
        timeseries_val = item["ts", "value"]
        unit = item.attrs["unit"] if item.attrs["unit"] != "None" else "1"
        scale = item.attrs["scale"] if item.attrs["scale"] != "None" else 1.0
        return Timeseries(timeseries_val, str_id, unit, scale)

    with hdf.File(logfile, "r") as hdf_file:
        timeseries = []
        str_ids = hdf_file.keys()
        # Map all hex ids to their string ids, as the string ids are the keys in the HDF5 file
        hex_to_str_id = { int(hdf_file.get(f'{key}').attrs["hex_id"]):key for key in str_ids }
        for canid in canids:
            key = f'"{canid}"'
            if key in str_ids:
                timeseries.append(to_timeseries(key, hdf_file))
                continue
            
            key = int(canid)
            if key in hex_to_str_id:
                timeseries.append(to_timeseries(hex_to_str_id[key], hdf_file))
                continue

            logging.warning(f"Could not find {canid} in log file!")

        return timeseries
                

def plot_all(timeseries: List[Timeseries]):
    ylabel = ""
    plt.figure(frameon=True)
    for ts in timeseries:
        # Data may be unsorted due to compression
        ts.timeseries.sort()
        plt.plot(ts.timeseries["ts"], ts.scale * ts.timeseries["value"], label=ts.str_id.strip('"'))
        ylabel += ts.unit if ylabel == "" else "/" + ts.unit

    plt.ylabel(ylabel)
    plt.legend()
    plt.show()

        
if __name__ == "__main__":
    logging.basicConfig(format="%(levelname)s: %(message)s")

    parser = argparse.ArgumentParser(description="Plot CAN data from HDF5 files")
    parser.add_argument("logfile", help="Path to HDF5 log", type=os_file)
    parser.add_argument("can_ids", nargs="+", help="CAN IDs to use for plotting")
    args = parser.parse_args()

    timeseries = get_data_for(args.logfile, args.can_ids)

    plot_all(timeseries)
    

