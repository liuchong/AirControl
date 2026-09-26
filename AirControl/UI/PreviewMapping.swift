import CoreGraphics

enum PreviewMapping {
    /// Vision 和视图都以左下角为原点、向上变大。预览层的
    /// `layerPointConverted(fromCaptureDevicePoint:)` 接收的是左上角原点，
    /// 并且会把画面顶部换成较小的视图 y。叠加层要把这个结果翻回视图坐标。
    /// 没有换算结果时，直接使用 Vision 自己的 y，并只镜像 x。
    static func viewPoint(
        visionX: CGFloat,
        visionY: CGFloat,
        bounds: CGRect,
        convertedLayerPoint: CGPoint?
    ) -> CGPoint {
        if let convertedLayerPoint,
           convertedLayerPoint.x.isFinite,
           convertedLayerPoint.y.isFinite,
           bounds.width > 1,
           bounds.height > 1 {
            return CGPoint(x: convertedLayerPoint.x, y: bounds.height - convertedLayerPoint.y)
        }
        return CGPoint(x: (1 - visionX) * bounds.width, y: visionY * bounds.height)
    }
}
