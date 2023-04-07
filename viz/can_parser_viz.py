import argparse
import os
import sys
import h5py as hdf
import logging
from pathlib import Path
from typing import List
import matplotlib.pyplot as plt
import numpy as np

class Timeseries:
    def __init__(self, timeseries, str_id, unit, scale):
        self.str_id = str_id
        self.timeseries = timeseries
        self.unit = unit
        self.scale = scale


def is_hdf5_file(path) -> bool:
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
        scale = item.attrs["scale"]
        return Timeseries(timeseries_val, str_id, unit, scale)

    with hdf.File(logfile, "r") as hdf_file:
        timeseries = []
        str_ids = list(hdf_file["CAN_IDs"].keys())

        # Map all hex ids to their string ids, as the string ids are the keys in the HDF5 file
        hex_to_str_id = { int(hdf_file["CAN_IDs"].get(key).attrs["hex_id"]):key for key in str_ids }
        for canid in canids:
            key = canid
            if key in str_ids:
                timeseries.append(to_timeseries(key, hdf_file["CAN_IDs"]))
                continue
            
            key = int(canid)
            if key in hex_to_str_id:
                timeseries.append(to_timeseries(hex_to_str_id[key], hdf_file["CAN_IDs"]))
                continue

            logging.warning(f"Could not find {canid} in log file!")

        return timeseries
                

def get_comments_for(logfile, comments):
    if comments is None:
        return
    
    with hdf.File(logfile, "r") as hdf_file:
        cmt_timeseries = []
        
        if not comments:
            # When no comment indices are given, plot all comments
            comments = hdf_file["COMMENTS"]["id"]
        
        for cmt_id in comments:
            cmt_id_index = int(cmt_id) - 1 # CAN log comments are indexed with 1
            cmt_timeseries.append([hdf_file["COMMENTS"][cmt_id_index][1], hdf_file["COMMENTS"][cmt_id_index][2].decode("UTF-8")])

    return cmt_timeseries


def plot_all(timeseries: List[Timeseries], comment):
    ylabel = ""
    plt.figure(frameon=True )
    plt.tight_layout()

    min_value = sys.maxsize
    max_value = -sys.maxsize
    for tms in timeseries:
        # Data may be unsorted due to compression
        tms.timeseries.sort()

        ts = tms.timeseries["ts"]
        value = tms.timeseries["value"] # * tms.scale
        plt.plot(ts, value, label=tms.str_id)
        
        prov_min = min(value)
        prov_max = max(value)
        if prov_min < min_value:
            min_value = prov_min
        if prov_max > max_value:
            max_value = prov_max

        ylabel += tms.unit if ylabel == "" else ", " + tms.unit
    

    for cmt in comments:
        x_pos = cmt[0]
        plt.vlines(x_pos, min_value, max_value * 1.02, linestyles="dotted", color="red")
        plt.text(x_pos, max_value * 1.025, cmt[1], horizontalalignment='center')

    plt.ylabel(ylabel)
    plt.ylim((min_value, max_value * 1.05))
    plt.legend()
    plt.show()

        
def main():
    logging.basicConfig(format="%(levelname)s: %(message)s")

    parser = argparse.ArgumentParser(description="Plot CAN data from HDF5 files")
    parser.add_argument("logfile", help="Path to HDF5 log", type=is_hdf5_file)
    parser.add_argument("can_ids", nargs="+", help="CAN IDs to use for plotting")
    parser.add_argument("-c", "--plot-comments", nargs="*", help="Plot comments as vertical lines. If no ID(s) specified, show all. Use space as a separator.")
    args = parser.parse_args()

    do_plot_comments = args.plot_comments is not None

    timeseries = get_data_for(args.logfile, args.can_ids)
    comments = get_comments_for(args.logfile, args.plot_comments) if do_plot_comments else []

    plot_all(timeseries, comments)
    

if __name__ == "__main__":
    main()