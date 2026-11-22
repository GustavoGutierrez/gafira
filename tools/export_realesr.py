#!/usr/bin/env python3
"""Exports `assets/models/realesr-general-x4v3.onnx` (spec 084, D-115).

The weights are the official ones of Real-ESRGAN (BSD-3-Clause):
https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesr-general-x4v3.pth
SHA-256 8dc7edb9ac80ccdc30c3a5dca6616509367f05fbc184ad95b731f05bece96292

Usage: export_realesr.py realesr-general-x4v3.pth realesr-general-x4v3.onnx
Needs torch and onnx; it is not part of the build.
"""

import sys

import torch
from torch import nn


class Compact(nn.Module):
    """The SRVGGNetCompact of Real-ESRGAN: 32 convolutions of 64 features."""

    def __init__(self, features=64, convolutions=32, scale=4):
        super().__init__()
        body = [nn.Conv2d(3, features, 3, 1, 1), nn.PReLU(features)]
        for _ in range(convolutions):
            body += [nn.Conv2d(features, features, 3, 1, 1), nn.PReLU(features)]
        body += [nn.Conv2d(features, 3 * scale * scale, 3, 1, 1)]
        self.body = nn.ModuleList(body)
        self.shuffle = nn.PixelShuffle(scale)
        self.scale = scale

    def forward(self, image):
        out = image
        for layer in self.body:
            out = layer(out)
        near = nn.functional.interpolate(image, scale_factor=self.scale, mode="nearest")
        return self.shuffle(out) + near


def main():
    source, target = sys.argv[1:3]
    weights = torch.load(source, map_location="cpu", weights_only=True)
    weights = weights.get("params", weights.get("params_ema", weights))
    model = Compact().eval()
    model.load_state_dict(weights, strict=True)
    torch.onnx.export(
        model,
        torch.rand(1, 3, 64, 64),
        target,
        input_names=["input"],
        output_names=["output"],
        opset_version=13,
        dynamic_axes={"input": {2: "h", 3: "w"}, "output": {2: "H", 3: "W"}},
    )


if __name__ == "__main__":
    main()
