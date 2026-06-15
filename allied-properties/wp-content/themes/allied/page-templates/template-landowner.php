<?php
/**
 * Template Name: Landowner — Sell Us Your Land
 *
 * The recommended production form is a standard plugin form (Contact Form 7 /
 * WPForms) embedded in the page content via shortcode — see HANDOFF.md for the
 * exact CF7 field config (incl. file upload + email notification).
 *
 * To guarantee a working lead form out of the box (and as a no-plugin
 * fallback), this template also ships a secure native handler: it validates a
 * nonce, sanitizes input, accepts an image/PDF upload, and emails the lead to
 * the site admin via wp_mail(). If the page content contains a form shortcode,
 * the native form is hidden automatically.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

/**
 * Native landowner submission handler (fallback / out-of-the-box).
 */
function allied_handle_landowner_submission() {
	$result = array( 'status' => '', 'message' => '' );

	if ( empty( $_POST['allied_landowner_submit'] ) ) {
		return $result;
	}
	if ( ! isset( $_POST['allied_landowner_nonce'] ) || ! wp_verify_nonce( wp_unslash( $_POST['allied_landowner_nonce'] ), 'allied_landowner' ) ) {
		return array( 'status' => 'error', 'message' => __( 'Security check failed. Please try again.', 'allied' ) );
	}
	// Honeypot.
	if ( ! empty( $_POST['allied_hp'] ) ) {
		return array( 'status' => 'success', 'message' => __( 'Thank you. We will be in touch.', 'allied' ) );
	}

	$name    = sanitize_text_field( wp_unslash( $_POST['name'] ?? '' ) );
	$email   = sanitize_email( wp_unslash( $_POST['email'] ?? '' ) );
	$phone   = sanitize_text_field( wp_unslash( $_POST['phone'] ?? '' ) );
	$parcel  = sanitize_text_field( wp_unslash( $_POST['parcel'] ?? '' ) );
	$acreage = sanitize_text_field( wp_unslash( $_POST['acreage'] ?? '' ) );
	$county  = sanitize_text_field( wp_unslash( $_POST['county'] ?? '' ) );
	$message = sanitize_textarea_field( wp_unslash( $_POST['message'] ?? '' ) );

	if ( ! $name || ! is_email( $email ) ) {
		return array( 'status' => 'error', 'message' => __( 'Please provide your name and a valid email address.', 'allied' ) );
	}

	// Handle optional file/photo upload.
	$attachments = array();
	if ( ! empty( $_FILES['parcel_file']['name'] ) ) {
		require_once ABSPATH . 'wp-admin/includes/file.php';
		$allowed = array( 'jpg|jpeg' => 'image/jpeg', 'png' => 'image/png', 'webp' => 'image/webp', 'pdf' => 'application/pdf' );
		$upload  = wp_handle_upload(
			$_FILES['parcel_file'],
			array( 'test_form' => false, 'mimes' => $allowed )
		);
		if ( isset( $upload['error'] ) ) {
			return array( 'status' => 'error', 'message' => sprintf( __( 'Upload error: %s', 'allied' ), $upload['error'] ) );
		}
		if ( ! empty( $upload['file'] ) ) {
			$attachments[] = $upload['file'];
		}
	}

	$to      = apply_filters( 'allied_landowner_recipient', get_option( 'admin_email' ) );
	$subject = sprintf( __( '[Land Lead] %1$s — %2$s', 'allied' ), $name, $county ?: $parcel );
	$body    = array(
		__( 'New landowner submission from the website:', 'allied' ),
		'',
		__( 'Name: ', 'allied' ) . $name,
		__( 'Email: ', 'allied' ) . $email,
		__( 'Phone: ', 'allied' ) . $phone,
		__( 'County: ', 'allied' ) . $county,
		__( 'Parcel ID / Address: ', 'allied' ) . $parcel,
		__( 'Approx. Acreage: ', 'allied' ) . $acreage,
		'',
		__( 'Message:', 'allied' ),
		$message,
	);
	$headers = array( 'Reply-To: ' . $name . ' <' . $email . '>' );

	$sent = wp_mail( $to, $subject, implode( "\n", $body ), $headers, $attachments );

	if ( $sent ) {
		return array( 'status' => 'success', 'message' => __( 'Thank you — your submission has been received. A member of our acquisitions team will follow up shortly.', 'allied' ) );
	}
	return array( 'status' => 'error', 'message' => __( 'We could not send your submission. Please email us directly or try again later.', 'allied' ) );
}

get_header();

$submission   = allied_handle_landowner_submission();
$has_shortcode = false;

while ( have_posts() ) :
	the_post();
	$content       = get_the_content();
	$has_shortcode = ( has_shortcode( $content, 'contact-form-7' ) || has_shortcode( $content, 'wpforms' ) );
	?>
	<section class="page-banner">
		<div class="container">
			<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Landowners', 'allied' ); ?></p>
			<h1><?php the_title(); ?></h1>
			<p class="lead"><?php echo esc_html( get_the_excerpt() ?: __( 'Have residential land in Northeastern North Carolina or Hampton Roads, Virginia? We would like to hear from you.', 'allied' ) ); ?></p>
		</div>
	</section>

	<section class="section">
		<div class="container split" style="align-items:start;">
			<div class="stack entry-content">
				<?php the_content(); ?>
				<?php if ( ! $content ) : ?>
					<h2><?php esc_html_e( 'A straightforward process', 'allied' ); ?></h2>
					<p class="text-muted"><?php esc_html_e( 'Tell us about your parcel and our acquisitions team will review it against our active markets. If it fits, we move quickly and professionally — from first conversation to a clean closing.', 'allied' ); ?></p>
					<ol>
						<li><?php esc_html_e( 'Submit your parcel details below.', 'allied' ); ?></li>
						<li><?php esc_html_e( 'Our team evaluates fit and reaches out.', 'allied' ); ?></li>
						<li><?php esc_html_e( 'We present terms and handle the rest.', 'allied' ); ?></li>
					</ol>
				<?php endif; ?>
			</div>

			<div>
				<?php if ( $submission['status'] ) : ?>
					<div class="notice <?php echo $submission['status'] === 'success' ? 'notice--info' : ''; ?>" role="status" style="margin-bottom:var(--space-md);<?php echo $submission['status'] === 'error' ? 'border-left:3px solid #b3261e;' : ''; ?>">
						<?php echo esc_html( $submission['message'] ); ?>
					</div>
				<?php endif; ?>

				<?php if ( ! $has_shortcode && $submission['status'] !== 'success' ) : ?>
					<form class="allied-form" method="post" enctype="multipart/form-data" novalidate>
						<?php wp_nonce_field( 'allied_landowner', 'allied_landowner_nonce' ); ?>
						<p style="position:absolute;left:-9999px;" aria-hidden="true">
							<label>Leave this empty<input type="text" name="allied_hp" tabindex="-1" autocomplete="off" /></label>
						</p>
						<div class="form-row">
							<div class="form-field"><label for="lo-name"><?php esc_html_e( 'Full name', 'allied' ); ?> <span class="req">*</span></label><input id="lo-name" type="text" name="name" required></div>
							<div class="form-field"><label for="lo-email"><?php esc_html_e( 'Email', 'allied' ); ?> <span class="req">*</span></label><input id="lo-email" type="email" name="email" required></div>
						</div>
						<div class="form-row">
							<div class="form-field"><label for="lo-phone"><?php esc_html_e( 'Phone', 'allied' ); ?></label><input id="lo-phone" type="tel" name="phone"></div>
							<div class="form-field"><label for="lo-county"><?php esc_html_e( 'County / City', 'allied' ); ?></label><input id="lo-county" type="text" name="county"></div>
						</div>
						<div class="form-row">
							<div class="form-field"><label for="lo-parcel"><?php esc_html_e( 'Parcel ID or address', 'allied' ); ?></label><input id="lo-parcel" type="text" name="parcel"></div>
							<div class="form-field"><label for="lo-acreage"><?php esc_html_e( 'Approx. acreage', 'allied' ); ?></label><input id="lo-acreage" type="text" name="acreage"></div>
						</div>
						<div class="form-field"><label for="lo-message"><?php esc_html_e( 'Tell us about the property', 'allied' ); ?></label><textarea id="lo-message" name="message"></textarea></div>
						<div class="form-field">
							<label for="lo-file"><?php esc_html_e( 'Upload a survey, plat, or photo (JPG, PNG, PDF)', 'allied' ); ?></label>
							<input id="lo-file" type="file" name="parcel_file" accept=".jpg,.jpeg,.png,.webp,.pdf">
						</div>
						<button class="btn btn--primary btn--lg" type="submit" name="allied_landowner_submit" value="1"><?php esc_html_e( 'Submit parcel details', 'allied' ); ?></button>
						<p class="form-note" style="margin-top:var(--space-sm);"><?php esc_html_e( 'Your information is sent directly to our acquisitions team and is never shared.', 'allied' ); ?></p>
					</form>
				<?php endif; ?>
			</div>
		</div>
	</section>
	<?php
endwhile;

get_footer();
