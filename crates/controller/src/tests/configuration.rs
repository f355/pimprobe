// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;

#[tokio::test]
async fn coordinate_readback_waits_for_its_trailing_probe_report() {
    let (_path, _listener, controller, mut socket) = ready().await;
    let before = controller.state().position;
    let mut task = tokio::spawn({
        let controller = controller.clone();
        async move {
            controller
                .write_coordinates(54, [Some(-12.5), None, None, None], Some(2.5))
                .await
        }
    });
    for command in ["G10 L2 P1 X-12.500 R2.500000\n", "$#\n"] {
        assert_eq!(
            frame::read_frame(&mut socket).await.unwrap(),
            (b'Q', command.as_bytes().to_vec())
        );
    }
    data(&mut socket, "[G54:-12.5,0,20,0]\n[WCS-R:54,2.5]\n").await;
    assert!(timeout(Duration::from_millis(20), &mut task).await.is_err());
    data(&mut socket, "[PROBE:0,0,0,0:0,0]\n").await;
    task.await.unwrap().unwrap();
    assert_eq!(controller.state().wcs_origin, Some([-12.5, 0., 20., 0.]));
    assert_eq!(controller.state().wcs_rotation, Some(2.5));
    assert_eq!(controller.state().position, before);
    controller.shutdown().await;
}

#[tokio::test]
async fn coordinate_writes_reject_unconfirmed_origins_and_rotation() {
    for (axes, rotation, reply, message) in [
        (
            [Some(-12.5), None, None, None],
            None,
            "[G54:-11,0,20,0]\n",
            "new work zero",
        ),
        (
            [None; 4],
            Some(2.5),
            "[WCS-R:54,3.5]\n",
            "work coordinate rotation",
        ),
    ] {
        let (_path, _listener, controller, mut socket) = ready().await;
        let task = tokio::spawn({
            let controller = controller.clone();
            async move { controller.write_coordinates(54, axes, rotation).await }
        });
        frame::read_frame(&mut socket).await.unwrap();
        assert_eq!(
            frame::read_frame(&mut socket).await.unwrap(),
            (b'Q', b"$#\n".to_vec())
        );
        data(&mut socket, reply).await;
        let error = task.await.unwrap().unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        controller.shutdown().await;
    }
}

#[tokio::test]
async fn modes_require_a_fresh_parser_report_not_an_acknowledgment() {
    let (_path, _listener, controller, mut socket) = ready().await;
    let mut task = tokio::spawn({
        let controller = controller.clone();
        async move { controller.read_modes().await }
    });
    assert_eq!(
        frame::read_frame(&mut socket).await.unwrap(),
        (b'Q', b"$G\n".to_vec())
    );
    data(&mut socket, "ok\n").await;
    assert!(timeout(Duration::from_millis(20), &mut task).await.is_err());
    data(&mut socket, "[GC:G20 G18 G91 G93]\n").await;
    assert_eq!(
        task.await.unwrap().unwrap(),
        Modes {
            units: 20,
            distance: 91,
            feed: 93
        }
    );
    assert_eq!(controller.state().plane, 18);
    controller.shutdown().await;
}

#[tokio::test]
async fn settings_read_and_write_wait_for_completed_replies() {
    let (_path, _listener, controller, mut socket) = ready().await;
    let mut task = tokio::spawn({
        let controller = controller.clone();
        async move {
            controller.write_setting(33, 2.25).await?;
            controller.read_settings().await
        }
    });
    assert_eq!(
        frame::read_frame(&mut socket).await.unwrap(),
        (b'Q', b"$33=2.250000\n".to_vec())
    );
    data(&mut socket, "$33=2.25\n").await;
    assert!(timeout(Duration::from_millis(20), &mut task).await.is_err());
    data(&mut socket, "ok\n").await;
    assert_eq!(
        frame::read_frame(&mut socket).await.unwrap(),
        (b'Q', b"$$\n".to_vec())
    );
    data(&mut socket, "$33=2.25\n$34=-7\n$35=-50\n").await;
    assert!(timeout(Duration::from_millis(20), &mut task).await.is_err());
    data(&mut socket, "$201=-40\n$202=-100\nok\n").await;
    let settings = task.await.unwrap().unwrap();
    assert_eq!(
        settings,
        [(33, 2.25), (34, -7.), (35, -50.), (201, -40.), (202, -100.)].into()
    );
    assert_eq!(controller.state().probe_offset, [2.25, -7., 10., 0.]);
    controller.shutdown().await;
}
